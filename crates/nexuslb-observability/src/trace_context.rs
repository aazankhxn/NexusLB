use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static GLOBAL_TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);
const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

/// Formats standard W3C traceparent into a fixed 55-byte stack array without any heap allocations
#[inline(always)]
pub fn format_traceparent(
    version: u8,
    trace_id: &[u8; 16],
    span_id: &[u8; 8],
    flags: u8,
) -> [u8; 55] {
    let mut out = [0u8; 55];
    out[0] = HEX_CHARS[(version >> 4) as usize];
    out[1] = HEX_CHARS[(version & 0x0f) as usize];
    out[2] = b'-';
    let mut idx = 3;
    for &b in trace_id {
        out[idx] = HEX_CHARS[(b >> 4) as usize];
        out[idx + 1] = HEX_CHARS[(b & 0x0f) as usize];
        idx += 2;
    }
    out[35] = b'-';
    idx = 36;
    for &b in span_id {
        out[idx] = HEX_CHARS[(b >> 4) as usize];
        out[idx + 1] = HEX_CHARS[(b & 0x0f) as usize];
        idx += 2;
    }
    out[52] = b'-';
    out[53] = HEX_CHARS[(flags >> 4) as usize];
    out[54] = HEX_CHARS[(flags & 0x0f) as usize];
    out
}

/// W3C TraceContext traceparent implementation following RFC specifications:
/// format: {version}-{trace_id}-{parent_id}-{trace_flags}
/// example: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceContext {
    pub version: u8,
    pub trace_id: [u8; 16],
    pub span_id: [u8; 8],
    pub flags: u8,
}

impl TraceContext {
    /// Create a new root trace context with high-precision timestamp and atomic sequence
    #[inline(always)]
    pub fn new_root(sampled: bool) -> Self {
        let counter = GLOBAL_TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut trace_id = [0u8; 16];
        trace_id[0..8].copy_from_slice(&now_nanos.to_be_bytes());
        trace_id[8..16].copy_from_slice(&counter.to_be_bytes());

        let mut span_id = [0u8; 8];
        let span_val = counter.wrapping_mul(0x9E3779B97F4A7C15);
        span_id.copy_from_slice(&(if span_val == 0 { 1u64 } else { span_val }).to_be_bytes());

        let flags = if sampled { 0x01 } else { 0x00 };

        Self {
            version: 0x00,
            trace_id,
            span_id,
            flags,
        }
    }

    /// Spawn a downstream child trace context preserving trace_id and flags, but generating a new span_id
    #[inline(always)]
    pub fn new_child(&self) -> Self {
        let counter = GLOBAL_TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut span_id = [0u8; 8];
        let span_val = counter.wrapping_mul(0x517cc1b727220a95);
        span_id.copy_from_slice(&(if span_val == 0 { 1u64 } else { span_val }).to_be_bytes());

        Self {
            version: self.version,
            trace_id: self.trace_id,
            span_id,
            flags: self.flags,
        }
    }

    /// Parse a W3C traceparent header string
    pub fn parse(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.trim().split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let version = u8::from_str_radix(parts[0], 16).ok()?;
        if version == 0xFF {
            return None; // 0xFF is invalid in W3C specification
        }

        if parts[1].len() != 32 {
            return None;
        }
        let mut trace_id = [0u8; 16];
        for i in 0..16 {
            trace_id[i] = u8::from_str_radix(&parts[1][i * 2..i * 2 + 2], 16).ok()?;
        }
        if trace_id == [0u8; 16] {
            return None; // All-zero trace_id is invalid
        }

        if parts[2].len() != 8 * 2 {
            return None;
        }
        let mut span_id = [0u8; 8];
        for i in 0..8 {
            span_id[i] = u8::from_str_radix(&parts[2][i * 2..i * 2 + 2], 16).ok()?;
        }
        if span_id == [0u8; 8] {
            return None; // All-zero span_id is invalid
        }

        let flags = u8::from_str_radix(parts[3], 16).ok()?;

        Some(Self {
            version,
            trace_id,
            span_id,
            flags,
        })
    }

    /// Returns whether this trace has the sampled flag set (0x01)
    #[inline(always)]
    pub fn is_sampled(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Formats trace context as standard W3C traceparent string
    #[inline(always)]
    pub fn to_header_value(&self) -> String {
        let bytes = format_traceparent(self.version, &self.trace_id, &self.span_id, self.flags);
        unsafe { String::from_utf8_unchecked(bytes.to_vec()) }
    }

    pub fn trace_id_hex(&self) -> String {
        hex_encode_16(&self.trace_id)
    }

    pub fn span_id_hex(&self) -> String {
        hex_encode_8(&self.span_id)
    }
}

impl fmt::Display for TraceContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_header_value())
    }
}

#[inline(always)]
fn hex_encode_16(bytes: &[u8; 16]) -> String {
    let mut out = [0u8; 32];
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = HEX_CHARS[(b >> 4) as usize];
        out[i * 2 + 1] = HEX_CHARS[(b & 0x0f) as usize];
    }
    unsafe { String::from_utf8_unchecked(out.to_vec()) }
}

#[inline(always)]
fn hex_encode_8(bytes: &[u8; 8]) -> String {
    let mut out = [0u8; 16];
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = HEX_CHARS[(b >> 4) as usize];
        out[i * 2 + 1] = HEX_CHARS[(b & 0x0f) as usize];
    }
    unsafe { String::from_utf8_unchecked(out.to_vec()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_parse_and_format() {
        let header = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let ctx = TraceContext::parse(header).expect("Failed to parse valid traceparent");
        assert_eq!(ctx.version, 0);
        assert!(ctx.is_sampled());
        assert_eq!(ctx.to_header_value(), header);
    }

    #[test]
    fn test_trace_context_child_generation() {
        let parent = TraceContext::new_root(true);
        let child = parent.new_child();

        assert_eq!(parent.trace_id, child.trace_id);
        assert_ne!(parent.span_id, child.span_id);
        assert_eq!(parent.flags, child.flags);
    }

    #[test]
    fn test_invalid_traceparent_headers() {
        assert!(TraceContext::parse("invalid-header").is_none());
        assert!(
            TraceContext::parse("00-00000000000000000000000000000000-00f067aa0ba902b7-01")
                .is_none()
        ); // zero trace_id
        assert!(
            TraceContext::parse("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01")
                .is_none()
        ); // zero span_id
        assert!(
            TraceContext::parse("ff-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
                .is_none()
        ); // version 0xff invalid
    }
}
