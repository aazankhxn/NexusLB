use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

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
    /// Create a new root trace context with randomly generated trace_id and span_id
    pub fn new_root(sampled: bool) -> Self {
        let mut rng = rand::thread_rng();
        let mut trace_id = [0u8; 16];
        let mut span_id = [0u8; 8];
        rng.fill(&mut trace_id);
        rng.fill(&mut span_id);

        // Ensure non-zero IDs
        if trace_id == [0u8; 16] {
            trace_id[0] = 1;
        }
        if span_id == [0u8; 8] {
            span_id[0] = 1;
        }

        let flags = if sampled { 0x01 } else { 0x00 };

        Self {
            version: 0x00,
            trace_id,
            span_id,
            flags,
        }
    }

    /// Spawn a downstream child trace context preserving trace_id and flags, but generating a new span_id
    pub fn new_child(&self) -> Self {
        let mut rng = rand::thread_rng();
        let mut child_span_id = [0u8; 8];
        rng.fill(&mut child_span_id);
        if child_span_id == [0u8; 8] {
            child_span_id[0] = 1;
        }

        Self {
            version: self.version,
            trace_id: self.trace_id,
            span_id: child_span_id,
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
    pub fn to_header_value(&self) -> String {
        format!(
            "{:02x}-{}-{}-{:02x}",
            self.version,
            hex_encode_16(&self.trace_id),
            hex_encode_8(&self.span_id),
            self.flags
        )
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
    let mut s = String::with_capacity(32);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[inline(always)]
fn hex_encode_8(bytes: &[u8; 8]) -> String {
    let mut s = String::with_capacity(16);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
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
