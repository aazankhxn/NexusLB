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

const CLIENT_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const CLIENT_BODY_TIMEOUT: Duration = Duration::from_secs(30);

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
        retry_policy: RetryPolicy,
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
            header_ranges: [(u16, u16, u16, u16); 96],
            header_count: usize,
        }

        let mut initial_host: Option<String> = None;

        let result: std::io::Result<()> = async {
            loop {
                // 1. Single-pass header reading and inspection with Slowloris defense timeout
                if total_read == 0 {
                    let n = match tokio::time::timeout(CLIENT_HEADER_TIMEOUT, client.read(&mut read_buf[..])).await {
                        Ok(res) => res?,
                        Err(_) => {
                            let err_resp = b"HTTP/1.1 408 Request Timeout\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Timeout: client header read timed out\n";
                            let _ = client.write_all(err_resp).await;
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "Client header read timed out",
                            ));
                        }
                    };
                    if n == 0 {
                        return Ok(()); // Client disconnected cleanly
                    }
                    metrics.add_bytes_received(n as u64);
                    total_read = n;
                }

                const MAX_HEADER_LIMIT: usize = 64 * 1024;
                if total_read >= MAX_HEADER_LIMIT {
                    let err_resp = b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n";
                    let _ = client.write_all(err_resp).await;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "HTTP headers exceeded maximum configured limit (64KB)",
                    ));
                }

                let parsed = loop {
                    let mut headers = [httparse::EMPTY_HEADER; 96];
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
                            let mut cl_count = 0usize;
                            let mut te_present = false;
                            let mut smuggling_detected = false;
                            let mut ws = false;
                            let mut close = false;
                            let mut tp_r = None;
                            let mut h_r = None;
                            let mut inm_r = None;
                            let mut fwd = false;
                            let mut header_ranges = [(0u16, 0u16, 0u16, 0u16); 96];
                            let header_count = req.headers.len().min(96);

                            for (i, h) in req.headers.iter().enumerate().take(96) {
                                let name_s = (h.name.as_ptr() as usize - buf_base) as u16;
                                let name_e = name_s + h.name.len() as u16;
                                let val_s = (h.value.as_ptr() as usize - buf_base) as u16;
                                let val_e = val_s + h.value.len() as u16;
                                header_ranges[i] = (name_s, name_e, val_s, val_e);

                                // Smuggling defense: check for illegal CR or LF inside header value
                                if h.value.iter().any(|&b| b == b'\r' || b == b'\n') {
                                    smuggling_detected = true;
                                }

                                if h.name.eq_ignore_ascii_case("content-length") {
                                    cl_count += 1;
                                    if cl_count > 1 {
                                        smuggling_detected = true;
                                    }
                                    if let Ok(s) = std::str::from_utf8(h.value) {
                                        cl = s.trim().parse::<usize>().unwrap_or(0);
                                    }
                                } else if h.name.eq_ignore_ascii_case("transfer-encoding") {
                                    te_present = true;
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

                            // RFC 9112 Section 6.3: A message MUST NOT contain both Transfer-Encoding and Content-Length.
                            // Proxies must reject such requests to prevent HTTP request smuggling desync.
                            if smuggling_detected || (cl_count > 1) || (cl_count > 0 && te_present) {
                                let err_resp = b"HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nBad Request (Smuggling Defense - RFC 9112)\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP request rejected due to request smuggling vulnerability defense (RFC 9112)",
                                ));
                            }

                            // RFC 7230 Section 3.3.1: Reject standalone chunked transfer encoding with 501 Not Implemented
                            // rather than silently dropping chunked request bodies and corrupting downstream framing.
                            if te_present {
                                let err_resp = b"HTTP/1.1 501 Not Implemented\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nTransfer-Encoding: chunked is not supported; Content-Length required\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP request rejected: Transfer-Encoding chunked is not supported",
                                ));
                            }

                            const MAX_BODY_LIMIT: usize = 16 * 1024 * 1024;
                            if cl > MAX_BODY_LIMIT {
                                let err_resp = b"HTTP/1.1 413 Payload Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nPayload Too Large\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP body exceeded maximum configured limit (16MB)",
                                ));
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
                            if total_read >= MAX_HEADER_LIMIT {
                                let err_resp = b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP headers exceeded maximum configured limit (64KB)",
                                ));
                            }
                            if total_read == read_buf.len() {
                                let new_len = (read_buf.len() * 2).min(MAX_HEADER_LIMIT);
                                read_buf.resize(new_len, 0);
                            }
                            let n = match tokio::time::timeout(CLIENT_HEADER_TIMEOUT, client.read(&mut read_buf[total_read..])).await {
                                Ok(res) => res?,
                                Err(_) => {
                                    let err_resp = b"HTTP/1.1 408 Request Timeout\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Timeout: header read timed out\n";
                                    let _ = client.write_all(err_resp).await;
                                    return Err(std::io::Error::new(
                                        std::io::ErrorKind::TimedOut,
                                        "Client header read timed out",
                                    ));
                                }
                            };
                            if n == 0 {
                                return Ok(()); // Client disconnected cleanly
                            }
                            metrics.add_bytes_received(n as u64);
                            total_read += n;
                        }
                        Err(e) => {
                            if matches!(e, httparse::Error::TooManyHeaders) || total_read >= MAX_HEADER_LIMIT {
                                let err_resp = b"HTTP/1.1 431 Request Header Fields Too Large\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Header Fields Too Large\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    "HTTP headers exceeded maximum header limit (64KB / too many headers)",
                                ));
                            } else {
                                let err_resp = b"HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nBad Request\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("HTTP parse error: {:?}", e),
                                ));
                            }
                        }
                    }
                };

                let req_total_len = parsed.header_len + parsed.body_len;

                // 2. Read initial part of body into buffer with body timeout
                while total_read < req_total_len && total_read < read_buf.len() {
                    let n = match tokio::time::timeout(CLIENT_BODY_TIMEOUT, client.read(&mut read_buf[total_read..])).await {
                        Ok(res) => res?,
                        Err(_) => {
                            let err_resp = b"HTTP/1.1 408 Request Timeout\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nRequest Timeout: body read timed out\n";
                            let _ = client.write_all(err_resp).await;
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "Client body read timed out",
                            ));
                        }
                    };
                    if n == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Client disconnected before sending complete body",
                        ));
                    }
                    metrics.add_bytes_received(n as u64);
                    total_read += n;
                }

                let initial_body_bytes = total_read.saturating_sub(parsed.header_len).min(parsed.body_len);
                let remaining_body_to_stream = parsed.body_len.saturating_sub(initial_body_bytes);

                let method = std::str::from_utf8(&read_buf[parsed.method_range.0..parsed.method_range.1]).unwrap_or("GET");
                let path = std::str::from_utf8(&read_buf[parsed.path_range.0..parsed.path_range.1]).unwrap_or("/");

                // RFC 9112 Section 3.2: Defend against keep-alive Host switching / misdirection
                let current_host = parsed.host_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok())
                    .map(|h| h.trim().to_ascii_lowercase());

                let mut host_changed = false;
                match &initial_host {
                    None => {
                        initial_host = current_host.clone();
                    }
                    Some(first) => {
                        if let Some(ref curr) = current_host {
                            if first != curr {
                                host_changed = true;
                            }
                        }
                    }
                }

                if host_changed {
                    let err_resp = b"HTTP/1.1 421 Misdirected Request\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nMisdirected Request: Host changed on persistent connection (RFC 9112)\n";
                    let _ = client.write_all(err_resp).await;
                    conn_pool.dec_active(backend.id());
                    return Ok(());
                }

                // 3. HTTP -> HTTPS 301 Redirect if configured (with CRLF, open-redirect & protocol-relative defense)
                if redirect_http_to_https && !is_tls {
                    let clean_host = current_host.as_deref()
                        .filter(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' || c == ':' || c == '[' || c == ']'))
                        .unwrap_or("localhost");

                    let mut clean_path = path.chars().filter(|&c| c >= ' ' && c != '\x7f').collect::<String>();
                    if !clean_path.starts_with('/') {
                        clean_path = format!("/{}", clean_path);
                    }
                    while clean_path.starts_with("//") {
                        clean_path.remove(0);
                    }

                    let redir = format!(
                        "HTTP/1.1 301 Moved Permanently\r\nLocation: https://{}{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        clean_host, clean_path
                    );
                    client.write_all(redir.as_bytes()).await?;
                    return Ok(());
                }

                // 4. Evaluate filter chain (JWT authentication, header rewrites)
                let mut filter_headers: Option<Vec<(String, String)>> = None;
                let mut filter_method: Option<String> = None;
                let mut filter_path: Option<String> = None;
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
                        FilterAction::Continue => {
                            filter_headers = Some(headers_vec);
                            filter_method = Some(m_str);
                            filter_path = Some(p_str);
                        }
                    }
                }

                // 5. Fast RFC 7234 HTTP Cache check (lock-free when cache is empty)
                // RFC 7234 Section 3.2: Shared caches MUST NOT serve cached responses to requests with Authorization headers
                let has_auth = parsed.header_ranges[..parsed.header_count]
                    .iter()
                    .any(|&(s, e, _, _)| {
                        read_buf[s as usize..e as usize].eq_ignore_ascii_case(b"authorization")
                    });

                let host_hdr = parsed.host_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok())
                    .unwrap_or("localhost");
                let if_none_match = parsed.inm_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok());

                let cache_start = Instant::now();
                let cache_result = if has_auth {
                    nexuslb_cache::CacheResult::Miss
                } else {
                    cache.get(method, host_hdr, path, if_none_match)
                };

                match cache_result {
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
                // Prevent per-connection memory bloat: if a large POST inflated req_bytes,
                // shrink it back for subsequent keep-alive requests on this connection.
                if req_bytes.capacity() > 65_536 {
                    req_bytes = Vec::with_capacity(2048);
                } else {
                    req_bytes.clear();
                }
                let incoming_tp = parsed.tp_range
                    .and_then(|(s, e)| std::str::from_utf8(&read_buf[s..e]).ok());

                if let Some(ref hdrs) = filter_headers {
                    let final_method = filter_method.as_deref().unwrap_or(method);
                    let final_path = filter_path.as_deref().unwrap_or(path);
                    req_bytes.extend_from_slice(final_method.as_bytes());
                    req_bytes.push(b' ');
                    req_bytes.extend_from_slice(final_path.as_bytes());
                    req_bytes.extend_from_slice(b" HTTP/1.1\r\n");

                    for (k, v) in hdrs {
                        if !k.eq_ignore_ascii_case("x-forwarded-for")
                            && !k.eq_ignore_ascii_case("x-forwarded-proto")
                            && !k.eq_ignore_ascii_case("traceparent")
                        {
                            req_bytes.extend_from_slice(k.as_bytes());
                            req_bytes.extend_from_slice(b": ");
                            req_bytes.extend_from_slice(v.as_bytes());
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
                } else if !parsed.has_fwd && parsed.header_len >= 2 {
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

                if initial_body_bytes > 0 {
                    req_bytes.extend_from_slice(&read_buf[parsed.header_len..parsed.header_len + initial_body_bytes]);
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
                    const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(30);
                    let is_retryable_req = remaining_body_to_stream == 0 && RetryPolicy::is_retryable_method(method);
                    let max_attempts = if is_retryable_req {
                        retry_policy.max_retries + 1
                    } else {
                        1
                    };
                    let mut attempts = 0;

                    let (status_code, is_server_close, total_sent, req_dur, upstream) = loop {
                        let timeout_fut = tokio::time::timeout(
                            UPSTREAM_TIMEOUT,
                            Self::forward_http_request(
                                &mut client,
                                cached_upstream.take(),
                                &req_bytes,
                                remaining_body_to_stream,
                                &backend,
                                &conn_pool,
                                &metrics,
                                &mut resp_buf,
                            ),
                        );

                        match timeout_fut.await {
                            Ok(Ok(res)) => {
                                if attempts + 1 < max_attempts && retry_policy.is_retryable_status(res.0) {
                                    attempts += 1;
                                    retry_policy.backoff(attempts).await;
                                    continue;
                                }
                                break res;
                            }
                            Ok(Err(e)) => {
                                if attempts + 1 < max_attempts {
                                    attempts += 1;
                                    retry_policy.backoff(attempts).await;
                                    continue;
                                }
                                let err_resp = b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nBad Gateway: upstream connection failed\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(e);
                            }
                            Err(_elapsed) => {
                                backend.stats().record_error();
                                metrics.inc_backend_errors();
                                if attempts + 1 < max_attempts {
                                    attempts += 1;
                                    retry_policy.backoff(attempts).await;
                                    continue;
                                }
                                let err_resp = b"HTTP/1.1 504 Gateway Timeout\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nGateway Timeout: upstream server failed to respond in time\n";
                                let _ = client.write_all(err_resp).await;
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    "Upstream request timed out after 30s",
                                ));
                            }
                        }
                    };

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
                    let initial_req_bytes_in_buf = parsed.header_len + initial_body_bytes;
                    let leftover = total_read.saturating_sub(initial_req_bytes_in_buf);
                    if leftover > 0 {
                        read_buf.copy_within(initial_req_bytes_in_buf..total_read, 0);
                    }
                    total_read = leftover;

                    if is_server_close || parsed.client_close {
                        // Upstream sent Connection: close or client requested close —
                        // drop the socket, but decrement active count to prevent drift
                        conn_pool.dec_active(backend.id());
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
        remaining_body: usize,
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

        // 2. Write initial payload (headers + initial body) to upstream; reconnect once if stale socket fails
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

        // 2b. Stream any remaining request body from client directly to upstream in chunks
        if remaining_body > 0 {
            let mut left = remaining_body;
            while left > 0 {
                let chunk_size = left.min(resp_buf.len());
                let n = match tokio::time::timeout(
                    CLIENT_BODY_TIMEOUT,
                    client.read(&mut resp_buf[..chunk_size]),
                )
                .await
                {
                    Ok(res) => res?,
                    Err(_) => {
                        backend.stats().record_error();
                        metrics.inc_backend_errors();
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "Client body stream read timed out",
                        ));
                    }
                };
                if n == 0 {
                    backend.stats().record_error();
                    metrics.inc_backend_errors();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Client disconnected before completing request body stream",
                    ));
                }
                metrics.add_bytes_received(n as u64);
                upstream.write_all(&resp_buf[..n]).await?;
                left -= n;
            }
        }

        // 3. Read response headers from upstream
        let mut total_read = 0;
        let mut header_len = 0;
        let mut content_length = None;
        let mut is_close = false;
        let mut is_chunked = false;
        let mut status_code = 200u16;

        loop {
            let n = upstream.read(&mut resp_buf[total_read..]).await?;
            if n == 0 {
                // If upstream sent EOF on first read of a cached connection, reconnect once
                if total_read == 0 {
                    trace!("Upstream sent EOF on first read, reconnecting");
                    const RECONNECT_TIMEOUT: Duration = Duration::from_secs(10);
                    upstream = conn_pool
                        .get_or_connect(backend.id(), backend.socket_addr())
                        .await
                        .map_err(|e| {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                            std::io::Error::new(std::io::ErrorKind::NotConnected, e.to_string())
                        })?;
                    match tokio::time::timeout(RECONNECT_TIMEOUT, upstream.write_all(initial_payload)).await {
                        Ok(res) => res.inspect_err(|_e| {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                        })?,
                        Err(_) => {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "Upstream reconnect write timed out",
                            ));
                        }
                    }
                    let n2 = match tokio::time::timeout(RECONNECT_TIMEOUT, upstream.read(&mut resp_buf[..])).await {
                        Ok(res) => res?,
                        Err(_) => {
                            backend.stats().record_error();
                            metrics.inc_backend_errors();
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::TimedOut,
                                "Upstream reconnect read timed out",
                            ));
                        }
                    };
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

            let mut headers = [httparse::EMPTY_HEADER; 96];
            let mut resp = httparse::Response::new(&mut headers);

            if let Ok(httparse::Status::Complete(hlen)) = resp.parse(&resp_buf[..total_read]) {
                header_len = hlen;
                status_code = resp.code.unwrap_or(200);
                for h in resp.headers.iter() {
                    if h.name.eq_ignore_ascii_case("content-length") {
                        if let Ok(s) = std::str::from_utf8(h.value) {
                            content_length = s.trim().parse::<usize>().ok();
                        }
                    } else if h.name.eq_ignore_ascii_case("transfer-encoding")
                        && h.value.eq_ignore_ascii_case(b"chunked")
                    {
                        is_chunked = true;
                    } else if h.name.eq_ignore_ascii_case("connection")
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

        if is_chunked {
            // Forward chunked streaming response until terminal "0\r\n\r\n" chunk
            // Mark is_close = true so this upstream socket is not reused across requests (prevent desync)
            is_close = true;
            let body_slice = &resp_buf[header_len..total_read];
            let already_terminated = body_slice.windows(5).any(|w| w == b"0\r\n\r\n")
                || body_slice.ends_with(b"0\r\n\r\n");

            if !already_terminated {
                loop {
                    let n = upstream.read(resp_buf).await?;
                    if n == 0 {
                        break;
                    }
                    client.write_all(&resp_buf[..n]).await?;
                    total_bytes += n as u64;

                    let chunk = &resp_buf[..n];
                    if chunk.windows(5).any(|w| w == b"0\r\n\r\n") || chunk.ends_with(b"0\r\n\r\n")
                    {
                        break;
                    }
                }
            }
        } else if is_close && content_length.is_none() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use nexuslb_core::backend::Backend;
    use nexuslb_core::types::{BackendAddress, BackendId, Protocol};

    fn test_backend() -> Arc<Backend> {
        let addr = "127.0.0.1:8080".parse().unwrap();
        Arc::new(Backend::new(
            BackendId::new(1),
            "test-backend",
            BackendAddress::new(addr),
            100,
            Protocol::Http1,
            None,
        ))
    }

    fn test_fixtures() -> (
        ConnectionPool,
        Arc<WorkerMetrics>,
        BufferPool,
        Arc<nexuslb_cache::HttpCache>,
        Arc<AccessLogger>,
        Arc<FilterChain>,
    ) {
        let conn_pool = ConnectionPool::new(nexuslb_network::ConnectionPoolConfig::default());
        let metrics = Arc::new(WorkerMetrics::new(0));
        let buffer_pool = BufferPool::new(16, 16384);
        let cache = Arc::new(nexuslb_cache::HttpCache::new(1024 * 1024));
        let (logger, _) = AccessLogger::new(false, "combined", "stdout");
        let access_logger = Arc::new(logger);
        let filter_chain = Arc::new(FilterChain::new());
        (
            conn_pool,
            metrics,
            buffer_pool,
            cache,
            access_logger,
            filter_chain,
        )
    }

    #[tokio::test]
    async fn test_smuggling_defense_cl_and_te_rejected() {
        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let backend = test_backend();
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let read_buf = buffer_pool.acquire();

        // Write malicious request containing BOTH Content-Length and Transfer-Encoding (RFC 9112 Sec 6.3)
        tokio::spawn(async move {
            let req = b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
            client_side.write_all(req).await.unwrap();

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("400 Bad Request"));
            assert!(resp_str.contains("Smuggling Defense"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(
            res.is_err(),
            "Must reject request with smuggling attack vector"
        );
    }

    #[tokio::test]
    async fn test_chunked_transfer_encoding_rejected_501() {
        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let backend = test_backend();
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let read_buf = buffer_pool.acquire();

        tokio::spawn(async move {
            let req = b"POST /upload HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n4\r\nWiki\r\n0\r\n\r\n";
            client_side.write_all(req).await.unwrap();

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("501 Not Implemented"));
            assert!(resp_str.contains("Transfer-Encoding: chunked is not supported"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_smuggling_defense_duplicate_content_length() {
        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let backend = test_backend();
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let read_buf = buffer_pool.acquire();

        // Write malicious request with duplicate Content-Length headers
        tokio::spawn(async move {
            let req = b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\nContent-Length: 10\r\n\r\nhello";
            client_side.write_all(req).await.unwrap();

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("400 Bad Request"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(res.is_err(), "Must reject duplicate Content-Length");
    }

    #[tokio::test]
    async fn test_payload_too_large_rejected() {
        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let backend = test_backend();
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let read_buf = buffer_pool.acquire();

        tokio::spawn(async move {
            // Content-Length is 20MB (exceeds default 16MB limit)
            let req =
                b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20971520\r\n\r\n";
            client_side.write_all(req).await.unwrap();

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("413 Payload Too Large"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(res.is_err(), "Must reject body > 16MB with 413");
    }

    #[tokio::test]
    async fn test_large_body_streaming_beyond_buffer() {
        // Spin up a mock backend listener
        let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();

        let backend = Arc::new(Backend::new(
            BackendId::new(99),
            "mock-backend",
            BackendAddress::new(backend_addr),
            100,
            Protocol::Http1,
            None,
        ));
        backend.set_state(nexuslb_core::types::BackendState::Up);

        // Spawn backend handler that expects a 64KB upload (larger than 16KB default buffer)
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = backend_listener.accept().await {
                let mut buf = vec![0u8; 100_000];
                let mut total = 0;
                while total < 65536 {
                    let n = socket.read(&mut buf[total..]).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    total += n;
                }
                // Respond 200 OK
                let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
                socket.write_all(resp).await.unwrap();
            }
        });

        let (mut client_side, server_side) = tokio::io::duplex(131072);
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let read_buf = buffer_pool.acquire();

        tokio::spawn(async move {
            let body_size = 65536usize;
            let header = format!(
                "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
                body_size
            );
            client_side.write_all(header.as_bytes()).await.unwrap();

            // Write 64KB body in chunks
            let chunk = vec![b'X'; 4096];
            for _ in 0..(body_size / 4096) {
                client_side.write_all(&chunk).await.unwrap();
            }

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("200 OK"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(
            res.is_ok(),
            "64KB body streamed successfully through 16KB buffer"
        );
    }

    #[tokio::test]
    async fn test_https_301_redirect_sanitization() {
        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let backend = test_backend();

        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let read_buf = buffer_pool.acquire();

        tokio::spawn(async move {
            // Test protocol-relative path traversal defense
            let req = b"GET ///evil.com/login HTTP/1.1\r\nHost: myapp.com\r\n\r\n";
            client_side.write_all(req).await.unwrap();

            let mut resp = vec![0u8; 1024];
            let n = client_side.read(&mut resp).await.unwrap();
            let resp_str = String::from_utf8_lossy(&resp[..n]);
            assert!(resp_str.contains("301 Moved Permanently"));
            assert!(resp_str.contains("Location: https://myapp.com/evil.com/login"));
            assert!(!resp_str.contains("Location: https:////evil.com"));
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false, // is_tls = false triggers redirect
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            true, // redirect_http_to_https = true
        )
        .await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_keepalive_host_switching_rejected_421() {
        let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend_listener.local_addr().unwrap();
        let backend = Arc::new(Backend::new(
            BackendId::new(2),
            "test_b",
            BackendAddress::new(backend_addr),
            100,
            Protocol::Http1,
            None,
        ));

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = backend_listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                let _ = socket.write_all(resp).await;
            }
        });

        let (conn_pool, metrics, buffer_pool, cache, access_logger, filter_chain) = test_fixtures();
        let (mut client_side, server_side) = tokio::io::duplex(4096);
        let read_buf = buffer_pool.acquire();

        tokio::spawn(async move {
            // First request on host-a
            client_side
                .write_all(b"GET /first HTTP/1.1\r\nHost: host-a.com\r\n\r\n")
                .await
                .unwrap();
            let mut resp1 = vec![0u8; 512];
            let n1 = client_side.read(&mut resp1).await.unwrap();
            assert!(String::from_utf8_lossy(&resp1[..n1]).contains("200 OK"));

            // Second request on same keep-alive TCP stream with DIFFERENT Host header
            client_side
                .write_all(b"GET /second HTTP/1.1\r\nHost: host-b.com\r\n\r\n")
                .await
                .unwrap();
            let mut resp2 = vec![0u8; 512];
            let n2 = client_side.read(&mut resp2).await.unwrap();
            let resp_str2 = String::from_utf8_lossy(&resp2[..n2]);
            assert!(
                resp_str2.contains("421 Misdirected Request"),
                "Expected 421 Misdirected Request on host switch, got: {}",
                resp_str2
            );
        });

        let res = HttpProxy::handle_connection(
            server_side,
            "127.0.0.1:12345".parse().unwrap(),
            backend,
            conn_pool,
            metrics,
            buffer_pool,
            RetryPolicy::default(),
            false,
            cache,
            read_buf,
            0,
            access_logger,
            filter_chain,
            false,
        )
        .await;

        assert!(res.is_ok());
    }
}
