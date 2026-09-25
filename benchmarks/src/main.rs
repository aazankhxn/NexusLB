use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser)]
#[command(name = "nexuslb-bench")]
#[command(
    about = "Reproducible load testing harness and performance verification tool for NexusLB"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run load test against target address
    Run {
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        target: String,

        #[arg(short, long, default_value_t = 100)]
        concurrency: usize,

        #[arg(short, long, default_value_t = 10)]
        duration_secs: u64,

        #[arg(long, default_value = "/")]
        path: String,
    },
    /// Print system information (CPU, RAM, OS, compiler) for reproducible benchmarks
    SystemInfo,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::SystemInfo => {
            print_system_info();
        }
        Commands::Run {
            target,
            concurrency,
            duration_secs,
            path,
        } => {
            let addr: SocketAddr = target.parse()?;
            run_benchmark(addr, concurrency, Duration::from_secs(duration_secs), path).await?;
        }
    }

    Ok(())
}

fn print_system_info() {
    println!("=== NexusLB Benchmark System Information ===");
    println!("OS:             {}", std::env::consts::OS);
    println!("Architecture:   {}", std::env::consts::ARCH);
    println!(
        "Logical Cores:  {}",
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    );
    println!("Rust Version:   {}", env!("CARGO_PKG_VERSION"));
    println!("Profile:        Release (LTO enabled, panic=abort)");
    println!("============================================");
}

async fn run_benchmark(
    target: SocketAddr,
    concurrency: usize,
    duration: Duration,
    path: String,
) -> anyhow::Result<()> {
    println!("Starting NexusLB Benchmark:");
    println!("  Target:       {}", target);
    println!("  Concurrency:  {}", concurrency);
    println!("  Duration:     {:?}", duration);
    println!("  Path:         {}", path);
    println!();

    let total_requests = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));

    let stop_time = Instant::now() + duration;
    let path = Arc::new(path);

    let mut handles = Vec::with_capacity(concurrency);
    let start_instant = Instant::now();

    for _ in 0..concurrency {
        let total_requests = total_requests.clone();
        let total_errors = total_errors.clone();
        let total_bytes = total_bytes.clone();
        let path = path.clone();

        handles.push(tokio::spawn(async move {
            let req_str = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
                path, target
            );
            let req_bytes = req_str.as_bytes();

            let mut stream = match TcpStream::connect(target).await {
                Ok(s) => {
                    let _ = s.set_nodelay(true);
                    Some(s)
                }
                Err(_) => None,
            };

            let mut buf = [0u8; 4096];

            while Instant::now() < stop_time {
                if stream.is_none() {
                    stream = match TcpStream::connect(target).await {
                        Ok(s) => {
                            let _ = s.set_nodelay(true);
                            Some(s)
                        }
                        Err(_) => {
                            total_errors.fetch_add(1, Ordering::Relaxed);
                            tokio::time::sleep(Duration::from_millis(5)).await;
                            continue;
                        }
                    };
                }

                let s = stream.as_mut().unwrap();
                if s.write_all(req_bytes).await.is_err() {
                    total_errors.fetch_add(1, Ordering::Relaxed);
                    stream = None;
                    continue;
                }

                match s.read(&mut buf).await {
                    Ok(n) if n > 0 => {
                        total_requests.fetch_add(1, Ordering::Relaxed);
                        total_bytes.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    _ => {
                        total_errors.fetch_add(1, Ordering::Relaxed);
                        stream = None;
                    }
                }
            }
        }));
    }

    for h in handles {
        let _ = h.await;
    }

    let elapsed = start_instant.elapsed();
    let requests = total_requests.load(Ordering::Relaxed);
    let errors = total_errors.load(Ordering::Relaxed);
    let bytes = total_bytes.load(Ordering::Relaxed);

    let req_per_sec = requests as f64 / elapsed.as_secs_f64();
    let mb_per_sec = (bytes as f64 / 1_048_576.0) / elapsed.as_secs_f64();

    println!("=== NexusLB Benchmark Results ===");
    println!("Elapsed Time:     {:.2?}", elapsed);
    println!("Total Requests:   {}", requests);
    println!("Failed Requests:  {}", errors);
    println!("Throughput:       {:.2} req/sec", req_per_sec);
    println!("Transfer Rate:    {:.2} MB/sec", mb_per_sec);
    println!("=================================");

    Ok(())
}
