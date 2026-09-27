use crossbeam::channel::{bounded, Receiver, RecvTimeoutError, Sender};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{self, BufWriter, Write};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLogFormat {
    Json,
    Combined,
}

impl AccessLogFormat {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "combined" | "apache" | "common" => Self::Combined,
            _ => Self::Json,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AccessLogEntry {
    pub timestamp_epoch_ms: u64,
    pub client_ip: String,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_us: u64,
    pub backend: String,
    pub bytes_sent: u64,
}

impl AccessLogEntry {
    pub fn new(
        client_ip: IpAddr,
        method: &str,
        path: &str,
        status: u16,
        latency: Duration,
        backend: &str,
        bytes_sent: u64,
    ) -> Self {
        let timestamp_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            timestamp_epoch_ms,
            client_ip: client_ip.to_string(),
            method: method.to_string(),
            path: path.to_string(),
            status,
            latency_us: latency.as_micros() as u64,
            backend: backend.to_string(),
            bytes_sent,
        }
    }

    pub fn format_combined(&self) -> String {
        // Formats as: <client_ip> - - [<timestamp>] "<method> <path> HTTP/1.1" <status> <bytes_sent> "<backend>" <latency_ms>ms
        let secs = self.timestamp_epoch_ms / 1000;
        let latency_ms = self.latency_us as f64 / 1000.0;
        format!(
            "{} - - [{}] \"{} {} HTTP/1.1\" {} {} \"{}\" {:.2}ms",
            self.client_ip,
            secs,
            self.method,
            self.path,
            self.status,
            self.bytes_sent,
            self.backend,
            latency_ms
        )
    }

    pub fn format_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[derive(Clone)]
pub struct AccessLogger {
    tx: Option<Sender<AccessLogEntry>>,
    enabled: bool,
    _running: Arc<AtomicBool>,
}

impl AccessLogger {
    pub fn disabled() -> Self {
        Self {
            tx: None,
            enabled: false,
            _running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn new(enabled: bool, format_str: &str, target: &str) -> (Self, Option<JoinHandle<()>>) {
        if !enabled {
            return (Self::disabled(), None);
        }

        let format = AccessLogFormat::parse(format_str);
        let (tx, rx): (Sender<AccessLogEntry>, Receiver<AccessLogEntry>) = bounded(131_072); // 128K non-blocking buffer
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let target_str = target.to_string();

        let handle = thread::Builder::new()
            .name("nexuslb-access-log".to_string())
            .spawn(move || {
                let mut writer: Box<dyn Write> = match target_str.trim().to_ascii_lowercase().as_str() {
                    "stderr" => Box::new(BufWriter::new(io::stderr())),
                    "stdout" => Box::new(BufWriter::new(io::stdout())),
                    file_path => match OpenOptions::new().create(true).append(true).open(file_path) {
                        Ok(f) => Box::new(BufWriter::new(f)),
                        Err(e) => {
                            eprintln!("Failed to open access log file '{}': {}, falling back to stdout", file_path, e);
                            Box::new(BufWriter::new(io::stdout()))
                        }
                    },
                };

                while running_clone.load(Ordering::Acquire) || !rx.is_empty() {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(entry) => {
                            let line = match format {
                                AccessLogFormat::Json => entry.format_json(),
                                AccessLogFormat::Combined => entry.format_combined(),
                            };
                            let _ = writeln!(writer, "{}", line);
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            let _ = writer.flush();
                        }
                        Err(RecvTimeoutError::Disconnected) => {
                            // All senders dropped, drain and terminate worker thread
                            break;
                        }
                    }
                }
                let _ = writer.flush();
            })
            .ok();

        (
            Self {
                tx: Some(tx),
                enabled: true,
                _running: running,
            },
            handle,
        )
    }

    #[inline(always)]
    pub fn log(&self, entry: AccessLogEntry) {
        if self.enabled {
            if let Some(ref tx) = self.tx {
                // Non-blocking try_send: if log queue is saturated under high load, drop log rather than stall dataplane
                let _ = tx.try_send(entry);
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Drop for AccessLogger {
    fn drop(&mut self) {
        self._running.store(false, Ordering::Release);
        // Explicitly drop channel sender to wake up and exit worker thread immediately
        self.tx.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_access_logger_thread_shutdown_on_drop() {
        let (logger, handle) = AccessLogger::new(true, "json", "stdout");
        assert!(logger.is_enabled());
        let handle = handle.expect("worker thread handle must exist");

        // Dropping logger must signal thread to exit
        drop(logger);

        // Joining must finish promptly without leaking thread
        let res = handle.join();
        assert!(res.is_ok(), "AccessLogger worker thread should cleanly terminate");
    }
}
