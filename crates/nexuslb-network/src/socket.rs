use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, Copy)]
pub struct SocketConfig {
    pub reuse_port: bool,
    pub tcp_nodelay: bool,
    pub recv_buffer_size: Option<usize>,
    pub send_buffer_size: Option<usize>,
    pub keepalive_idle: Option<Duration>,
    pub keepalive_interval: Option<Duration>,
    pub keepalive_retries: Option<u32>,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            reuse_port: true,
            tcp_nodelay: true,
            recv_buffer_size: Some(256 * 1024), // 256KB
            send_buffer_size: Some(256 * 1024), // 256KB
            keepalive_idle: Some(Duration::from_secs(60)),
            keepalive_interval: Some(Duration::from_secs(10)),
            keepalive_retries: Some(3),
        }
    }
}

pub fn create_listener(addr: SocketAddr, config: &SocketConfig) -> io::Result<TcpListener> {
    let domain = if addr.is_ipv6() {
        Domain::IPV6
    } else {
        Domain::IPV4
    };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

    socket.set_nonblocking(true)?;
    socket.set_reuse_address(true)?;

    #[cfg(all(unix, not(target_os = "solaris")))]
    if config.reuse_port {
        let _ = socket.set_reuse_port(true);
    }

    if let Some(rcv) = config.recv_buffer_size {
        let _ = socket.set_recv_buffer_size(rcv);
    }
    if let Some(snd) = config.send_buffer_size {
        let _ = socket.set_send_buffer_size(snd);
    }

    socket.bind(&addr.into())?;
    // Backlog 16384 for high concurrency bursts (matches kernel somaxconn)
    socket.listen(16384)?;

    // TCP_FASTOPEN: allow SYN+data for faster connection establishment (Linux only)
    #[cfg(target_os = "linux")]
    {
        let _ = socket.set_tcp_fastopen(256);
    }

    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}

pub fn configure_stream(stream: &TcpStream, config: &SocketConfig) -> io::Result<()> {
    stream.set_nodelay(config.tcp_nodelay)?;

    // We can also configure socket2 options via std stream if needed
    Ok(())
}
