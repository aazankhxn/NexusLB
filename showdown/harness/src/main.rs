use clap::Parser;
use hdrhistogram::Histogram;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(name = "showdown-harness")]
#[command(about = "High-precision head-to-head load balancer benchmarking harness")]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    target: String,

    #[arg(short, long, default_value_t = 100)]
    concurrency: usize,

    #[arg(short, long, default_value_t = 5)]
    duration_secs: u64,

    #[arg(long, default_value = "/")]
    path: String,

    #[arg(long)]
    json: bool,
}

#[derive(Serialize, Debug)]
pub struct BenchmarkReport {
    pub target: String,
    pub concurrency: usize,
    pub duration_secs: f64,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub rps: f64,
    pub transfer_mb_per_sec: f64,
    pub latency_min_us: u64,
    pub latency_mean_us: f64,
    pub latency_p50_us: u64,
    pub latency_p90_us: u64,
    pub latency_p99_us: u64,
    pub latency_max_us: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let addr: SocketAddr = args.target.parse()?;

    let total_requests = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));

    let stop_time = Instant::now() + Duration::from_secs(args.duration_secs);
    let path = Arc::new(args.path.clone());

    let mut handles = Vec::with_capacity(args.concurrency);
    let start_instant = Instant::now();

    for _ in 0..args.concurrency {
        let total_requests = total_requests.clone();
        let total_errors = total_errors.clone();
        let total_bytes = total_bytes.clone();
        let path = path.clone();

        handles.push(tokio::spawn(async move {
            let req_str = format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: keep-alive\r\n\r\n",
                path, addr
            );
            let req_bytes = req_str.as_bytes();

            let mut stream = match TcpStream::connect(addr).await {
                Ok(s) => {
                    let _ = s.set_nodelay(true);
                    Some(s)
                }
                Err(_) => None,
            };

            let mut local_latencies = Vec::with_capacity(16384);
            let mut buf = [0u8; 4096];

            while Instant::now() < stop_time {
                if stream.is_none() {
                    stream = match TcpStream::connect(addr).await {
                        Ok(s) => {
                            let _ = s.set_nodelay(true);
                            Some(s)
                        }
                        Err(_) => {
                            total_errors.fetch_add(1, Ordering::Relaxed);
                            tokio::time::sleep(Duration::from_millis(2)).await;
                            continue;
                        }
                    };
                }

                let req_start = Instant::now();
                let s = stream.as_mut().unwrap();

                if s.write_all(req_bytes).await.is_err() {
                    total_errors.fetch_add(1, Ordering::Relaxed);
                    stream = None;
                    continue;
                }

                match tokio::time::timeout(Duration::from_millis(500), s.read(&mut buf)).await {
                    Ok(Ok(n)) if n > 0 => {
                        let latency_us = req_start.elapsed().as_micros() as u64;
                        local_latencies.push(latency_us);
                        total_requests.fetch_add(1, Ordering::Relaxed);
                        total_bytes.fetch_add(n as u64, Ordering::Relaxed);
                    }
                    _ => {
                        total_errors.fetch_add(1, Ordering::Relaxed);
                        stream = None;
                    }
                }
            }

            local_latencies
        }));
    }

    let mut merged_hist = Histogram::<u64>::new_with_bounds(1, 60_000_000, 3)?;

    for h in handles {
        let latencies = h.await?;
        for l in latencies {
            let _ = merged_hist.record(l.max(1));
        }
    }

    let elapsed = start_instant.elapsed();
    let requests = total_requests.load(Ordering::Relaxed);
    let errors = total_errors.load(Ordering::Relaxed);
    let bytes = total_bytes.load(Ordering::Relaxed);

    let rps = requests as f64 / elapsed.as_secs_f64();
    let mb_per_sec = (bytes as f64 / 1_048_576.0) / elapsed.as_secs_f64();

    let report = BenchmarkReport {
        target: args.target.clone(),
        concurrency: args.concurrency,
        duration_secs: elapsed.as_secs_f64(),
        total_requests: requests,
        failed_requests: errors,
        rps,
        transfer_mb_per_sec: mb_per_sec,
        latency_min_us: merged_hist.min(),
        latency_mean_us: merged_hist.mean(),
        latency_p50_us: merged_hist.value_at_quantile(0.50),
        latency_p90_us: merged_hist.value_at_quantile(0.90),
        latency_p99_us: merged_hist.value_at_quantile(0.99),
        latency_max_us: merged_hist.max(),
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("--------------------------------------------------");
        println!("  Target:        {}", report.target);
        println!("  Concurrency:   {}", report.concurrency);
        println!("  Duration:      {:.2}s", report.duration_secs);
        println!("  Throughput:    {:.2} req/sec", report.rps);
        println!("  Transfer Rate: {:.2} MB/sec", report.transfer_mb_per_sec);
        println!("  Total Reqs:    {} (Failed: {})", report.total_requests, report.failed_requests);
        println!("  Latency (µs):  p50={}µs | p90={}µs | p99={}µs | max={}µs",
            report.latency_p50_us, report.latency_p90_us, report.latency_p99_us, report.latency_max_us);
        println!("--------------------------------------------------");
    }

    Ok(())
}
