use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct MockHttpBackend {
    addr: SocketAddr,
    request_count: Arc<AtomicU64>,
}

impl MockHttpBackend {
    pub async fn start(response_body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let request_count = Arc::new(AtomicU64::new(0));

        let count_clone = request_count.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let count = count_clone.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 2048];
                        if let Ok(n) = stream.read(&mut buf).await {
                            if n > 0 {
                                count.fetch_add(1, Ordering::Relaxed);
                                let resp = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                    response_body.len(),
                                    response_body
                                );
                                let _ = stream.write_all(resp.as_bytes()).await;
                            }
                        }
                    });
                }
            }
        });

        Self {
            addr,
            request_count,
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn request_count(&self) -> u64 {
        self.request_count.load(Ordering::Relaxed)
    }
}
