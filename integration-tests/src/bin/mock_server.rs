use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = std::env::args()
        .nth(1)
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9001);

    let name = std::env::args()
        .nth(2)
        .unwrap_or_else(|| format!("backend-{}", port));

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    let listener = TcpListener::bind(addr).await?;
    println!("Mock backend '{}' listening on {}", name, addr);

    let request_count = Arc::new(AtomicU64::new(0));
    let name = Arc::new(name);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let _ = socket.set_nodelay(true);
        let count = request_count.clone();
        let name_str = name.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            let mut total_read = 0;

            loop {
                // Parse complete HTTP request header
                let (header_len, content_length) = loop {
                    let mut headers = [httparse::EMPTY_HEADER; 64];
                    let mut req = httparse::Request::new(&mut headers);

                    match req.parse(&buf[..total_read]) {
                        Ok(httparse::Status::Complete(hlen)) => {
                            let mut cl = None;
                            for h in req.headers.iter() {
                                if h.name.eq_ignore_ascii_case("content-length") {
                                    if let Ok(s) = std::str::from_utf8(h.value) {
                                        cl = s.trim().parse::<usize>().ok();
                                    }
                                }
                            }
                            break (hlen, cl);
                        }
                        Ok(httparse::Status::Partial) => {
                            if total_read == buf.len() {
                                return;
                            }
                            let n = match socket.read(&mut buf[total_read..]).await {
                                Ok(n) if n > 0 => n,
                                _ => return,
                            };
                            total_read += n;
                        }
                        Err(_) => return,
                    }
                };

                let body_len = content_length.unwrap_or(0);
                let req_total_len = header_len + body_len;

                while total_read < req_total_len {
                    if total_read == buf.len() {
                        return;
                    }
                    let n = match socket.read(&mut buf[total_read..]).await {
                        Ok(n) if n > 0 => n,
                        _ => return,
                    };
                    total_read += n;
                }

                // Shift leftover bytes forward for next request
                let leftover = total_read - req_total_len;
                if leftover > 0 {
                    buf.copy_within(req_total_len..total_read, 0);
                }
                total_read = leftover;

                let c = count.fetch_add(1, Ordering::Relaxed) + 1;
                let body = format!(
                    "{{\"backend\":\"{}\",\"request\":{},\"status\":\"ok\"}}\n",
                    name_str, c
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{}",
                    body.len(),
                    body
                );

                if socket.write_all(resp.as_bytes()).await.is_err() {
                    break;
                }
            }
        });
    }
}
