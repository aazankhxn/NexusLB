use std::io;
use tokio::net::TcpStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpliceStats {
    pub bytes_client_to_backend: u64,
    pub bytes_backend_to_client: u64,
    pub zero_copy_used: bool,
}

/// Linux kernel zero-copy pipe abstraction
#[cfg(target_os = "linux")]
pub struct SplicePipe {
    pub reader: i32,
    pub writer: i32,
}

#[cfg(target_os = "linux")]
impl SplicePipe {
    pub fn create(pipe_size: usize) -> io::Result<Self> {
        let mut fds = [0i32; 2];
        let res = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
        if res < 0 {
            return Err(io::Error::last_os_error());
        }

        let (reader, writer) = (fds[0], fds[1]);
        if pipe_size > 0 {
            unsafe {
                libc::fcntl(writer, libc::F_SETPIPE_SZ, pipe_size as libc::c_int);
            }
        }

        Ok(Self { reader, writer })
    }
}

#[cfg(target_os = "linux")]
impl Drop for SplicePipe {
    fn drop(&mut self) {
        unsafe {
            if self.reader >= 0 {
                libc::close(self.reader);
            }
            if self.writer >= 0 {
                libc::close(self.writer);
            }
        }
    }
}

/// Zero-copy splice forwarder engine with Linux kernel splice acceleration and
/// cross-platform duplex streaming fallback.
#[derive(Clone)]
pub struct SpliceEngine {
    #[allow(dead_code)]
    pipe_size: usize,
}

impl Default for SpliceEngine {
    fn default() -> Self {
        Self {
            pipe_size: 64 * 1024, // 64KB kernel pipe buffer
        }
    }
}

impl SpliceEngine {
    pub fn new(pipe_size: usize) -> Self {
        Self { pipe_size }
    }

    pub fn pipe_size(&self) -> usize {
        self.pipe_size
    }

    /// Splices data bidirectionally between client and backend streams.
    /// On Linux, attempts kernel zero-copy splice through anonymous pipe pairs.
    /// On other platforms or fallback, uses optimized asynchronous duplex copying.
    pub async fn splice_bidirectional(
        &self,
        client: &mut TcpStream,
        backend: &mut TcpStream,
    ) -> io::Result<SpliceStats> {
        #[cfg(target_os = "linux")]
        {
            match self.splice_linux(client, backend).await {
                Ok(stats) => return Ok(stats),
                Err(err)
                    if err.raw_os_error() == Some(libc::ENOSYS)
                        || err.raw_os_error() == Some(libc::EINVAL) =>
                {
                    tracing::debug!("Linux splice syscall unsupported or invalid, falling back to duplex copy: {}", err);
                }
                Err(err) => return Err(err),
            }
        }

        self.fallback_copy(client, backend).await
    }

    #[cfg(target_os = "linux")]
    async fn splice_linux(
        &self,
        client: &mut TcpStream,
        backend: &mut TcpStream,
    ) -> io::Result<SpliceStats> {
        use std::os::unix::io::AsRawFd;

        let pipe_up = SplicePipe::create(self.pipe_size)?;
        let pipe_down = SplicePipe::create(self.pipe_size)?;

        let client_fd = client.as_raw_fd();
        let backend_fd = backend.as_raw_fd();

        let pipe_up_r = pipe_up.reader;
        let pipe_up_w = pipe_up.writer;
        let pipe_down_r = pipe_down.reader;
        let pipe_down_w = pipe_down.writer;

        let (mut client_read, mut client_write) = client.split();
        let (mut backend_read, mut backend_write) = backend.split();

        // Forward: client -> pipe_up -> backend
        let fwd = async {
            let mut total = 0u64;
            loop {
                client_read.readable().await?;
                let n = unsafe {
                    libc::splice(
                        client_fd,
                        std::ptr::null_mut(),
                        pipe_up_w,
                        std::ptr::null_mut(),
                        65536,
                        libc::SPLICE_F_NONBLOCK | libc::SPLICE_F_MOVE,
                    )
                };

                if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    return Err(err);
                }
                if n == 0 {
                    break;
                }

                backend_write.writable().await?;
                let written = unsafe {
                    libc::splice(
                        pipe_up_r,
                        std::ptr::null_mut(),
                        backend_fd,
                        std::ptr::null_mut(),
                        n as usize,
                        libc::SPLICE_F_NONBLOCK | libc::SPLICE_F_MOVE,
                    )
                };

                if written < 0 {
                    return Err(io::Error::last_os_error());
                }
                total += written as u64;
            }
            Ok::<u64, io::Error>(total)
        };

        // Reverse: backend -> pipe_down -> client
        let rev = async {
            let mut total = 0u64;
            loop {
                backend_read.readable().await?;
                let n = unsafe {
                    libc::splice(
                        backend_fd,
                        std::ptr::null_mut(),
                        pipe_down_w,
                        std::ptr::null_mut(),
                        65536,
                        libc::SPLICE_F_NONBLOCK | libc::SPLICE_F_MOVE,
                    )
                };

                if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        continue;
                    }
                    return Err(err);
                }
                if n == 0 {
                    break;
                }

                client_write.writable().await?;
                let written = unsafe {
                    libc::splice(
                        pipe_down_r,
                        std::ptr::null_mut(),
                        client_fd,
                        std::ptr::null_mut(),
                        n as usize,
                        libc::SPLICE_F_NONBLOCK | libc::SPLICE_F_MOVE,
                    )
                };

                if written < 0 {
                    return Err(io::Error::last_os_error());
                }
                total += written as u64;
            }
            Ok::<u64, io::Error>(total)
        };

        let (c2b, b2c) = tokio::try_join!(fwd, rev)?;

        Ok(SpliceStats {
            bytes_client_to_backend: c2b,
            bytes_backend_to_client: b2c,
            zero_copy_used: true,
        })
    }

    async fn fallback_copy(
        &self,
        client: &mut TcpStream,
        backend: &mut TcpStream,
    ) -> io::Result<SpliceStats> {
        let (c2b, b2c) = tokio::io::copy_bidirectional(client, backend).await?;
        Ok(SpliceStats {
            bytes_client_to_backend: c2b,
            bytes_backend_to_client: b2c,
            zero_copy_used: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_splice_duplex_loopback() {
        let engine = SpliceEngine::default();

        let listener_a = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_a = listener_a.local_addr().unwrap();

        let listener_b = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr_b = listener_b.local_addr().unwrap();

        // Spawn client A and client B
        let client_task = tokio::spawn(async move {
            let mut stream = TcpStream::connect(addr_a).await.unwrap();
            stream.write_all(b"PING_FROM_A").await.unwrap();
            stream.shutdown().await.unwrap();

            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.unwrap();
            buf
        });

        let backend_task = tokio::spawn(async move {
            let (mut stream, _) = listener_b.accept().await.unwrap();
            let mut buf = vec![0u8; 11];
            stream.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"PING_FROM_A");

            stream.write_all(b"PONG_FROM_B").await.unwrap();
            stream.shutdown().await.unwrap();
        });

        // Accept from listener A, connect to listener B, and splice between them
        let (mut conn_a, _) = listener_a.accept().await.unwrap();
        let mut conn_b = TcpStream::connect(addr_b).await.unwrap();

        let stats = engine
            .splice_bidirectional(&mut conn_a, &mut conn_b)
            .await
            .unwrap();

        assert_eq!(stats.bytes_client_to_backend, 11);
        assert_eq!(stats.bytes_backend_to_client, 11);

        let received = client_task.await.unwrap();
        assert_eq!(&received, b"PONG_FROM_B");
        backend_task.await.unwrap();
    }
}
