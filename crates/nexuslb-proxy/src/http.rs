use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::trace;

use crate::retry::RetryPolicy;
use crate::tcp::TcpProxy;
use nexuslb_core::backend::Backend;
use nexuslb_metrics::WorkerMetrics;
use nexuslb_network::{BufferPool, ConnectionPool};
use nexuslb_observability::{format_traceparent, AccessLogEntry, AccessLogger, ProxySpan};
use nexuslb_wasm::{FilterAction, FilterChain};

pub struct HttpProxy;

impl HttpProxy {
    /// Proxy an HTTP/1.1 or WebSocket request from client to selected backend with keep-alive
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_connection<S>(
        mut client: S,
        client_addr: SocketAddr,
        backend: Arc<Backend>,
        conn_pool: ConnectionPool,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
        _retry_policy: RetryPolicy,
        is_tls: bool,
        cache: Arc<nexuslb_cache::HttpCache>,
        mut read_buf: nexuslb_network::PooledBuffer,
        initial_read: usize,
        access_logger: Arc<AccessLogger>,
        filter_chain: Arc<FilterChain>,
        redirect_http_to_https: bool,
    ) -> std::io::Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        backend.stats().inc_active_connections();
        struct ConnGuard(Arc<Backend>);
        impl Drop for ConnGuard {
            fn drop(&mut self) {
                self.0.stats().dec_active_connections();
            }
        }
        let _conn_guard = ConnGuard(backend.clone());

        let mut resp_buf = buffer_pool.acquire();

        // Reusable request assembly buffer — allocated once per connection, cleared per request.
        let mut req_bytes = Vec::with_capacity(2048);
        let mut total_read = initial_read;

        // Pre-compute client-specific headers once per connection using stack buffer
        let mut xff_stack = [0u8; 80]; // "X-Forwarded-For: <ipv6_max>\r\n" fits in ~60 bytes
        let xff_len = {
            let mut cursor = std::io::Cursor::new(&mut xff_stack[..]);
            let _ = write!(cursor, "X-Forwarded-For: {}\r\n", client_addr.ip());
            cursor.position() as usize
        };
        let x_forwarded_for = &xff_stack[..xff_len];

        let x_forwarded_proto: &[u8] = if is_tls {
            b"X-Forwarded-Proto: https\r\n"
        } else {
            b"X-Forwarded-Proto: http\r\n"
        };

        let mut cached_upstream: Option<TcpStream> = None;

        struct ParsedReq {
            method_range: (usize, usize),
            path_range: (usize, usize),
            header_len: usize,
            body_len: usize,
            is_ws: bool,
            client_close: bool,
            tp_range: Option<(usize, usize)>,
            host_range: Option<(usize, usize)>,
            inm_range: Option<(usize, usize)>,
            has_fwd: bool,
            header_ranges: [(u16, u16, u16, u16); 32],
            header_count: usize,
        }

        let result: std::io::Result<()> = async {
            loop {
                // 1. Single-pass header reading and inspection
                if total_read == 0 {
                    let n = client.read(&mut read_buf[..]).await?;
                    if n == 0 {
                        return Ok(()); // Client disconnected cleanly
                    }
                    metrics.add_bytes_received(n as u64);
                    total_read = n;
                }

                let parsed = loop {
                    let mut headers = [httparse::EMPTY_HEADER; 32];
                    let mut req = httparse::Request::new(&mut headers);
                    match req.parse(&read_buf[..total_read]) {
                        Ok(httparse::Status::Complete(hlen)) => {
                            let buf_base = read_buf.as_ptr() as usize;
                            let m_ref = req.method.unwrap_or("GET");
                            let m_s = m_ref.as_ptr() as usize - buf_base;
                            let method_range = (m_s, m_s + m_ref.len());

                            let p_ref = req.path.unwrap_or("/");
                            let p_s = p_ref.as_ptr() as usize - buf_base;
                            let path_range = (p_s, p_s + p_ref.len());

                            let mut cl = 0usize;
                            let mut ws = false;
                            let mut close = false;
                            let mut tp_r = None;
                            let mut h_r = None;
                            let mut inm_r = None;
                            let mut fwd = false;
                            let mut header_ranges = [(0u16, 0u16, 0u16, 0u16); 32];
                            let header_count = req.headers.len().min(32);

                            for (i, h) in req.headers.iter().enumerate().take(32) {
                                let name_s = (h.name.as_ptr() as usize - buf_base) as u16;
                                let name_e = name_s + h.name.len() as u16;
                                let val_s = (h.value.as_ptr() as usize - buf_base) as u16;
                                let val_e = val_s + h.value.len() as u16;
                                header_ranges[i] = (name_s, name_e, val_s, val_e);

                                if h.name.eq_ignore_ascii_case("content-length") {
                                    if let Ok(s) = std::str::from_utf8(h.value) {
                                        cl = s.trim().parse::<usize>().unwrap_or(0);
                                    }
                                } else if h.name.eq_ignore_ascii_case("connection")
                                    && h.value.eq_ignore_ascii_case(b"close")
                                {
                                    close = true;
                                } else if h.name.eq_ignore_ascii_case("upgrade")
                                    && h.value.eq_ignore_ascii_case(b"websocket")
                                {
                                    ws = true;
                                } else if h.name.eq_ignore_ascii_case("traceparent") {
                                    fwd = true;
                                    tp_r = Some((val_s as usize, val_e as usize));
                                } else if h.name.eq_ignore_ascii_case("x-forwarded-for")
                                    || h.name.eq_ignore_ascii_case("x-forwarded-proto")
                                {
                                    fwd = true;
                                } else if h.name.eq_ignore_ascii_case("host") {
                                    h_r = Some((val_s as usize, val_e as usize));
                                } else if h.name.eq_ignore_ascii_case("if-none-match") {
                                    inm_r = Some((val_s as usize, val_e as usize));
                                }
                            }

                            break ParsedReq {
                                method_range,
                                path_range,
                                header_len: hlen,
                                body_len: cl,
                                is_ws: ws,
                                client_close: close,
                                tp_range: tp_r,
                                host_range: h_r,
                                inm_range: inm_r,
                                has_fwd: fwd,
                                header_ranges,
                                header_count,
                            };
                        }
                        Ok(httparse::Status::Partial) => {
                            if total_read == read_buf.len() {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP headers exceeded buffer size",
                                ));
                            }
                            let n = client.read(&mut read_buf[total_read..]).await?;
                            if n == 0 {
                                return Ok(()); // Client disconnected cleanly
                            }
                            metrics.add_bytes_received(n as u64);
                            total_read += n;
                        }
                        Err(e) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("HTTP parse error: {:?}", e),
                            ));
                        }
                    }
                };

                let req_total_len = parsed.header_len + parsed.body_len;

                // 2. Read remainder of body into buffer BEFORE borrowing header slices
                while total_read < req_total_len {
                    if total_read == read_buf.len() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "HTTP body exceeded buffer size",
                        ));
                    }
                    let n = client.read(&mut read_buf[total_read..]).await?;
                    if n == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Client disconnected before sending complete body",
                        ));
                    }
                    metrics.add_bytes_received(n as u64);
                    total_read += n;
                }

                let method = std::str::from_utf8(&read_buf[parsed.method_range.0..parsed.method_range.1]).unwrap_or("GET");
                let path = std::str::from_utf8(&read_buf[parsed.path_range.0..parsed.path_range.1]).unwrap_or("/");

                // 3. HTTP -> HTTPS 301 Redirect if configured
                if redirect_http_to_https && !is_tls {
                    let host_hdr = parsed.host_range
                        .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok())
                        .unwrap_or("localhost");
                    let redir = format!(
                        "HTTP/1.1 301 Moved Permanently\r\nLocation: https://{}{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        host_hdr, path
                    );
                    client.write_all(redir.as_bytes()).await?;
                    return Ok(());
                }

                // 4. Evaluate filter chain (JWT authentication, header rewrites)
                if !filter_chain.is_empty() {
                    let mut m_str = method.to_string();
                    let mut p_str = path.to_string();
                    let mut headers_vec = Vec::new();
                    for &(name_s, name_e, val_s, val_e) in &parsed.header_ranges[..parsed.header_count] {
                        if let (Ok(k), Ok(v)) = (
                            std::str::from_utf8(&read_buf[name_s as usize..name_e as usize]),
                            std::str::from_utf8(&read_buf[val_s as usize..val_e as usize]),
                        ) {
                            headers_vec.push((k.to_string(), v.to_string()));
                        }
                    }

                    match filter_chain.execute_request(&mut m_str, &mut p_str, &mut headers_vec) {
                        FilterAction::StopAndReply { status, body, .. } => {
                            let resp = format!(
                                "HTTP/1.1 {} Authorization Required\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                status,
                                body.len()
                            );
                            client.write_all(resp.as_bytes()).await?;
                            if !body.is_empty() {
                                client.write_all(&body).await?;
                            }
                            return Ok(());
                        }
                        FilterAction::Drop => {
                            return Ok(());
                        }
                        FilterAction::Continue => {}
                    }
                }

                // 5. Fast RFC 7234 HTTP Cache check (lock-free when cache is empty)
                let host_hdr = parsed.host_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok())
                    .unwrap_or("localhost");
                let if_none_match = parsed.inm_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok());

                let cache_start = Instant::now();
                match cache.get(method, host_hdr, path, if_none_match) {
                    nexuslb_cache::CacheResult::Hit(cached) => {
                        let resp_len = {
                            let mut cursor = std::io::Cursor::new(&mut resp_buf[..]);
                            let _ = write!(
                                cursor,
                                "HTTP/1.1 {} OK\r\nAge: {}\r\nX-Cache: HIT\r\nContent-Length: {}\r\n",
                                cached.status,
                                cached.age_secs(),
                                cached.body.len()
                            );
                            if let Some(ref tag) = cached.etag {
                                let _ = write!(&mut cursor, "ETag: {}\r\n", tag);
                            }
                            for (k, v) in &cached.headers {
                                if !k.eq_ignore_ascii_case("content-length")
                                    && !k.eq_ignore_ascii_case("age")
                                    && !k.eq_ignore_ascii_case("etag")
                                {
                                    let _ = write!(&mut cursor, "{}: {}\r\n", k, v);
                                }
                            }
                            let _ = std::io::Write::write_all(&mut cursor, b"\r\n");
                            cursor.position() as usize
                        };
                        client.write_all(&resp_buf[..resp_len]).await?;
                        client.write_all(&cached.body).await?;
                        metrics.inc_requests();
                        let total_sent = (resp_len + cached.body.len()) as u64;
                        metrics.add_bytes_sent(total_sent);

                        access_logger.log(AccessLogEntry::new(
                            client_addr.ip(),
                            method,
                            path,
                            cached.status,
                            cache_start.elapsed(),
                            "cache",
                            total_sent,
                        ));

                        let leftover = total_read.saturating_sub(req_total_len);
                        if leftover > 0 {
                            read_buf.copy_within(req_total_len..total_read, 0);
                        }
                        total_read = leftover;

                        if parsed.client_close {
                            return Ok(());
                        }
                        continue;
                    }
                    nexuslb_cache::CacheResult::NotModified(etag) => {
                        let resp_len = {
                            let mut cursor = std::io::Cursor::new(&mut resp_buf[..]);
                            let _ = std::io::Write::write_all(&mut cursor, b"HTTP/1.1 304 Not Modified\r\nX-Cache: HIT-REVALIDATED\r\n");
                            if let Some(ref tag) = etag {
                                let _ = write!(&mut cursor, "ETag: {}\r\n", tag);
                            }
                            let _ = std::io::Write::write_all(&mut cursor, b"\r\n");
                            cursor.position() as usize
                        };
                        client.write_all(&resp_buf[..resp_len]).await?;
                        metrics.inc_requests();
                        let total_sent = resp_len as u64;
                        metrics.add_bytes_sent(total_sent);

                        access_logger.log(AccessLogEntry::new(
                            client_addr.ip(),
                            method,
                            path,
                            304,
                            cache_start.elapsed(),
                            "cache",
                            total_sent,
                        ));

                        let leftover = total_read.saturating_sub(req_total_len);
                        if leftover > 0 {
                            read_buf.copy_within(req_total_len..total_read, 0);
                        }
                        total_read = leftover;

                        if parsed.client_close {
                            return Ok(());
                        }
                        continue;
                    }
                    nexuslb_cache::CacheResult::Miss => {}
                }

                // 6. Build forwarded request into reusable req_bytes Vec
                req_bytes.clear();
                let incoming_tp = parsed.tp_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok());

                if !parsed.has_fwd && parsed.header_len >= 2 {
                    // Ultra-fast zero-alloc slice forward: reuse original parsed headers directly!
                    req_bytes.extend_from_slice(&read_buf[..parsed.header_len - 2]);
                    req_bytes.extend_from_slice(x_forwarded_for);
                    req_bytes.extend_from_slice(x_forwarded_proto);
                    if let Some(tp) = incoming_tp {
                        let span = ProxySpan::new(Some(tp));
                        let tp_bytes = format_traceparent(
                            span.trace_context.version,
                            &span.trace_context.trace_id,
                            &span.trace_context.span_id,
                            span.trace_context.flags,
                        );
                        req_bytes.extend_from_slice(b"traceparent: ");
                        req_bytes.extend_from_slice(&tp_bytes);
                        req_bytes.extend_from_slice(b"\r\n\r\n");
                    } else {
                        req_bytes.extend_from_slice(b"\r\n");
                    }
                } else {
                    req_bytes.extend_from_slice(method.as_bytes());
                    req_bytes.push(b' ');
                    req_bytes.extend_from_slice(path.as_bytes());
                    req_bytes.extend_from_slice(b" HTTP/1.1\r\n");

                    for &(name_s, name_e, val_s, val_e) in &parsed.header_ranges[..parsed.header_count] {
                        let name_bytes = &read_buf[name_s as usize..name_e as usize];
                        if !name_bytes.eq_ignore_ascii_case(b"x-forwarded-for")
                            && !name_bytes.eq_ignore_ascii_case(b"x-forwarded-proto")
                            && !name_bytes.eq_ignore_ascii_case(b"traceparent")
                        {
                            req_bytes.extend_from_slice(name_bytes);
                            req_bytes.extend_from_slice(b": ");
                            req_bytes.extend_from_slice(&read_buf[val_s as usize..val_e as usize]);
                            req_bytes.extend_from_slice(b"\r\n");
                        }
                    }

                    req_bytes.extend_from_slice(x_forwarded_for);
                    req_bytes.extend_from_slice(x_forwarded_proto);
                    if let Some(tp) = incoming_tp {
                        let span = ProxySpan::new(Some(tp));
                        let tp_bytes = format_traceparent(
                            span.trace_context.version,
                            &span.trace_context.trace_id,
                            &span.trace_context.span_id,
                            span.trace_context.flags,
                        );
                        req_bytes.extend_from_slice(b"traceparent: ");
                        req_bytes.extend_from_slice(&tp_bytes);
                        req_bytes.extend_from_slice(b"\r\n\r\n");
                    } else {
                        req_bytes.extend_from_slice(b"\r\n");
                    }
                }

                if parsed.body_len > 0 {
                    req_bytes.extend_from_slice(&read_buf[parsed.header_len..req_total_len]);
                }

                if parsed.is_ws {
                    let upstream = match cached_upstream.take() {
                        Some(s) => s,
                        None => conn_pool
                            .get_or_connect(backend.id(), backend.socket_addr())
                            .await
                            .map_err(|e| {
                                std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                            })?,
                    };
                    return Self::handle_websocket_upgrade(
                        client,
                        upstream,
                        &req_bytes,
                        backend.clone(),
                        metrics,
                        buffer_pool,
                    )
                    .await;
                } else {
                    let (status_code, is_server_close, total_sent, req_dur, upstream) = Self::forward_http_request(
                        &mut client,
                        cached_upstream.take(),
                        &req_bytes,
                        &backend,
                        &conn_pool,
                        &metrics,
                        &mut resp_buf,
                    )
                    .await?;

                    access_logger.log(AccessLogEntry::new(
                        client_addr.ip(),
                        method,
                        path,
                        status_code,
                        req_dur,
                        backend.name(),
                        total_sent,
                    ));

                    // Shift leftover bytes in read_buf to front
                    let leftover = total_read.saturating_sub(req_total_len);
                    if leftover > 0 {
                        read_buf.copy_within(req_total_len..total_read, 0);
                    }
                    total_read = leftover;

                    if is_server_close || parsed.client_close {
                        return Ok(());
                    } else {
                        cached_upstream = Some(upstream);
                    }
                }
            }
        }.await;

        // Return upstream connection to pool if still alive
        if let Some(upstream) = cached_upstream {
            conn_pool.return_connection(backend.id(), upstream);
        }

        result
    }

    async fn handle_websocket_upgrade<S>(
        client: S,
        mut upstream: TcpStream,
        initial_payload: &[u8],
        backend: Arc<Backend>,
        metrics: Arc<WorkerMetrics>,
        buffer_pool: BufferPool,
    ) -> std::io::Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        trace!("Handling WebSocket handshake upgrade");
        upstream.write_all(initial_payload).await?;
        TcpProxy::forward(client, upstream, backend, metrics, buffer_pool).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn forward_http_request<S>(
        client: &mut S,
        existing_upstream: Option<TcpStream>,
        initial_payload: &[u8],
        backend: &Backend,
        conn_pool: &ConnectionPool,
        metrics: &WorkerMetrics,
        resp_buf: &mut [u8],
    ) -> std::io::Result<(u16, bool, u64, Duration, TcpStream)>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let start = Instant::now();
        metrics.inc_requests();
        backend.stats().inc_requests();

        // 1. Acquire upstream connection (reusing session-cached connection if available)
        let mut upstream = match existing_upstream {
            Some(s) => s,
            None => conn_pool
                .get_or_connect(backend.id(), backend.socket_addr())
                .await
                .map_err(|e| {
                    backend.stats().record_error();
                    metrics.inc_backend_errors();
                    std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                })?,
        };

        // 2. Write request to upstream; reconnect once if stale socket fails
        if let Err(e) = upstream.write_all(initial_payload).await {
            trace!("Cached upstream write error ({}), reconnecting", e);
            upstream = conn_pool
                .get_or_connect(backend.id(), backend.socket_addr())
                .await
                .map_err(|e| {
                    backend.stats().record_error();
                    metrics.inc_backend_errors();
                    std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                })?;
            upstream
                .write_all(initial_payload)
                .await
                .inspect_err(|_e| {
                    backend.stats().record_error();
                    metrics.inc_backend_errors();
                })?;
        }

        // 3. Read response headers from upstream
        let mut total_read = 0;
        let mut header_len = 0;
        let mut content_length = None;
        let mut is_close = false;
        let mut status_code = 200u16;

        loop {
            let n = upstream.read(&mut resp_buf[total_read..]).await?;
            if n == 0 {
                // If upstream sent EOF on first read of a cached connection, reconnect once
                if total_read == 0 {
                    trace!("Upstream sent EOF on first read, reconnecting");
                    upstream = conn_pool
                        .get_or_connect(backend.id(), backend.socket_addr())
                        .await
                        .map_err(|e| {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                            std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                        })?;
                    upstream
                        .write_all(initial_payload)
                        .await
                        .inspect_err(|_e| {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                        })?;
                    let n2 = upstream.read(&mut resp_buf[..]).await?;
                    if n2 == 0 {
                        backend.stats().record_error();
                        metrics.inc_backend_errors();
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Upstream closed connection",
                        ));
                    }
                    total_read = n2;
                } else {
                    break;
                }
            } else {
                total_read += n;
            }

            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut resp = httparse::Response::new(&mut headers);

            if let Ok(httparse::Status::Complete(hlen)) = resp.parse(&resp_buf[..total_read]) {
                header_len = hlen;
                status_code = resp.code.unwrap_or(200);
                for h in resp.headers.iter() {
                    if h.name.eq_ignore_ascii_case("content-length") {
                        if let Ok(s) = std::str::from_utf8(h.value) {
                            content_length = s.trim().parse::<usize>().ok();
                        }
                    }
                    if h.name.eq_ignore_ascii_case("connection")
                        && h.value.eq_ignore_ascii_case(b"close")
                    {
                        is_close = true;
                    }
                }
                break;
            }

            if total_read == resp_buf.len() {
                break;
            }
        }

        // 4. If Content-Length is present, consolidate remaining body into buffer for single TCP write
        let mut remaining = 0usize;
        if let Some(cl) = content_length {
            let body_read = total_read.saturating_sub(header_len);
            remaining = cl.saturating_sub(body_read);
            let needed = total_read + remaining;

            while remaining > 0 && needed <= resp_buf.len() {
                let n = upstream.read(&mut resp_buf[total_read..needed]).await?;
                if n == 0 {
                    break;
                }
                total_read += n;
                remaining -= n;
            }
        }

        // Single write of headers + body directly to client
        if total_read > 0 {
            client.write_all(&resp_buf[..total_read]).await?;
        }

        let mut total_bytes = total_read as u64;

        // 5. For oversized bodies larger than buffer, stream remaining bytes
        while remaining > 0 {
            let to_read = remaining.min(resp_buf.len());
            let n = upstream.read(&mut resp_buf[..to_read]).await?;
            if n == 0 {
                break;
            }
            client.write_all(&resp_buf[..n]).await?;
            remaining -= n;
            total_bytes += n as u64;
        }

        if is_close && content_length.is_none() {
            // No Content-Length specified: read until EOF
            loop {
                let n = upstream.read(resp_buf).await?;
                if n == 0 {
                    break;
                }
                client.write_all(&resp_buf[..n]).await?;
                total_bytes += n as u64;
            }
        }

        let duration = start.elapsed();
        backend
            .stats()
            .record_success(duration, initial_payload.len() as u64, total_bytes);
        metrics.record_latency(duration);
        metrics.add_bytes_sent(total_bytes);

        Ok((status_code, is_close, total_bytes, duration, upstream))
    }
}
