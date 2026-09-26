use crate::trace_context::TraceContext;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// High-precision distributed tracing span representing the lifecycle of an individual request through NexusLB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanReport {
    pub trace_id: String,
    pub span_id: String,
    pub sampled: bool,
    pub duration_micros: u64,
    pub route: Option<String>,
    pub backend_id: Option<u64>,
    pub backend_addr: Option<String>,
    pub status_code: Option<u16>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

pub struct ProxySpan {
    pub trace_context: TraceContext,
    start_time: Instant,
    route: Option<String>,
    backend_id: Option<u64>,
    backend_addr: Option<SocketAddr>,
    status_code: Option<u16>,
    bytes_sent: u64,
    bytes_received: u64,
}

impl ProxySpan {
    /// Create a new proxy span from an existing upstream trace context or start a new root
    pub fn new(incoming_traceparent: Option<&str>) -> Self {
        let trace_context = incoming_traceparent
            .and_then(TraceContext::parse)
            .map(|parent| parent.new_child())
            .unwrap_or_else(|| TraceContext::new_root(true));

        Self {
            trace_context,
            start_time: Instant::now(),
            route: None,
            backend_id: None,
            backend_addr: None,
            status_code: None,
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn set_route(&mut self, route: impl Into<String>) {
        self.route = Some(route.into());
    }

    pub fn set_backend(&mut self, id: u64, addr: SocketAddr) {
        self.backend_id = Some(id);
        self.backend_addr = Some(addr);
    }

    pub fn set_status(&mut self, status_code: u16) {
        self.status_code = Some(status_code);
    }

    pub fn add_bytes(&mut self, sent: u64, received: u64) {
        self.bytes_sent += sent;
        self.bytes_received += received;
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Finish span and return a serializable report
    pub fn finish(self) -> SpanReport {
        let duration = self.start_time.elapsed();
        SpanReport {
            trace_id: self.trace_context.trace_id_hex(),
            span_id: self.trace_context.span_id_hex(),
            sampled: self.trace_context.is_sampled(),
            duration_micros: duration.as_micros() as u64,
            route: self.route,
            backend_id: self.backend_id,
            backend_addr: self.backend_addr.map(|a| a.to_string()),
            status_code: self.status_code,
            bytes_sent: self.bytes_sent,
            bytes_received: self.bytes_received,
        }
    }
}
