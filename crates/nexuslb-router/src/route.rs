use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathMatch {
    Exact(String),
    Prefix(String),
    Any,
}

#[inline(always)]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h1), Some(h2)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                decoded.push((h1 << 4) | h2);
                i += 3;
                continue;
            }
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Canonicalize HTTP request paths by resolving '.', '..', duplicate slashes,
/// decoding percent-encoded traversal sequences (RFC 3986 / CWE-22), and stripping query/fragment.
pub fn normalize_path(raw_path: &str) -> String {
    let path_only = raw_path.split('?').next().unwrap_or(raw_path);
    let path_only = path_only.split('#').next().unwrap_or(path_only);

    let mut current = path_only.to_string();
    for _ in 0..2 {
        let next = percent_decode(&current);
        if next == current {
            break;
        }
        current = next;
    }

    let mut segments = Vec::new();
    for seg in current.split('/') {
        let s = seg.trim();
        match s {
            "" | "." => continue,
            ".." => {
                segments.pop();
            }
            valid => segments.push(valid),
        }
    }

    let mut normalized = String::with_capacity(current.len() + 1);
    normalized.push('/');
    normalized.push_str(&segments.join("/"));
    normalized
}

impl PathMatch {
    #[inline(always)]
    pub fn matches(&self, path: &str) -> bool {
        match self {
            PathMatch::Exact(expected) => {
                let norm = normalize_path(path);
                let exp_norm = normalize_path(expected);
                norm == exp_norm
            }
            PathMatch::Prefix(prefix) => {
                let norm = normalize_path(path);
                let pref_norm = normalize_path(prefix);
                if pref_norm == "/" {
                    true
                } else if norm == pref_norm {
                    true
                } else if let Some(remainder) = norm.strip_prefix(&pref_norm) {
                    remainder.starts_with('/') || remainder.is_empty()
                } else {
                    false
                }
            }
            PathMatch::Any => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostMatch {
    Exact(String),
    Suffix(String), // e.g. .example.com
    Any,
}

impl HostMatch {
    #[inline(always)]
    pub fn matches(&self, host: &str) -> bool {
        // Strip port from host header if present: api.example.com:8080 -> api.example.com
        let host_no_port = host.split(':').next().unwrap_or(host);
        match self {
            HostMatch::Exact(expected) => host_no_port.eq_ignore_ascii_case(expected),
            HostMatch::Suffix(suffix) => {
                let norm_suffix = suffix.trim_start_matches('.').to_ascii_lowercase();
                let host_lower = host_no_port.to_ascii_lowercase();
                host_lower == norm_suffix || host_lower.ends_with(&format!(".{}", norm_suffix))
            }
            HostMatch::Any => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderMatch {
    pub name: String,
    pub value: String,
}

impl HeaderMatch {
    #[inline(always)]
    pub fn matches(&self, headers: &[(String, String)]) -> bool {
        for (k, v) in headers {
            if k.eq_ignore_ascii_case(&self.name) && v == &self.value {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub name: String,
    pub host: HostMatch,
    pub path: PathMatch,
    pub methods: Option<Vec<String>>,
    pub headers: Option<Vec<HeaderMatch>>,
    pub sni: Option<String>,
    pub pool_name: String,
    pub priority: i32,
}

impl Route {
    #[inline(always)]
    pub fn matches(
        &self,
        host: Option<&str>,
        path: &str,
        method: Option<&str>,
        headers: Option<&[(String, String)]>,
        sni: Option<&str>,
    ) -> bool {
        if let Some(h) = host {
            if !self.host.matches(h) {
                return false;
            }
        } else if !matches!(self.host, HostMatch::Any) {
            return false;
        }

        if !self.path.matches(path) {
            return false;
        }

        if let (Some(expected_methods), Some(m)) = (&self.methods, method) {
            if !expected_methods
                .iter()
                .any(|expected| expected.eq_ignore_ascii_case(m))
            {
                return false;
            }
        }

        if let (Some(expected_headers), Some(h_list)) = (&self.headers, headers) {
            for expected in expected_headers {
                if !expected.matches(h_list) {
                    return false;
                }
            }
        }

        if let (Some(expected_sni), Some(actual_sni)) = (&self.sni, sni) {
            if !expected_sni.eq_ignore_ascii_case(actual_sni) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_match_suffix_boundary_defense() {
        let matcher = HostMatch::Suffix("example.com".to_string());

        // Valid subdomains & exact domain
        assert!(matcher.matches("example.com"));
        assert!(matcher.matches("api.example.com"));
        assert!(matcher.matches("sub.service.example.com"));
        assert!(matcher.matches("API.EXAMPLE.COM:8080"));

        // Malicious domain suffix attacks MUST be rejected
        assert!(!matcher.matches("evilexample.com"));
        assert!(!matcher.matches("not-example.com"));
        assert!(!matcher.matches("attacker_example.com"));
        assert!(!matcher.matches("badexample.com:443"));

        // Leading dot suffix definition also works correctly
        let matcher_dot = HostMatch::Suffix(".corp.internal".to_string());
        assert!(matcher_dot.matches("corp.internal"));
        assert!(matcher_dot.matches("auth.corp.internal"));
        assert!(!matcher_dot.matches("fakecorp.internal"));
    }

    #[test]
    fn test_path_match_query_and_fragment_normalization() {
        let exact = PathMatch::Exact("/api/v1/users".to_string());
        assert!(exact.matches("/api/v1/users"));
        assert!(exact.matches("/api/v1/users?page=1&limit=10"));
        assert!(exact.matches("/api/v1/users#section2"));
        assert!(exact.matches("/api/v1/users?redirect=/admin#top"));
        assert!(!exact.matches("/api/v1/users/create"));

        let prefix = PathMatch::Prefix("/static".to_string());
        assert!(prefix.matches("/static/app.js"));
        assert!(prefix.matches("/static/bundle.css?v=1.2.3"));
        assert!(!prefix.matches("/other/path"));
    }

    #[test]
    fn test_path_match_traversal_and_slash_normalization() {
        let prefix = PathMatch::Prefix("/admin".to_string());
        // Traversal and duplicate slash attacks must normalize and match correctly
        assert!(prefix.matches("//admin/dashboard"));
        assert!(prefix.matches("/public/../admin/users"));
        assert!(prefix.matches("/api/%2e%2e/admin"));
        assert!(prefix.matches("/%61dmin/settings"));

        // Non-matching paths must not match
        assert!(!prefix.matches("/administrator"));
        assert!(!prefix.matches("/public/user"));

        let exact = PathMatch::Exact("/api/secret".to_string());
        assert!(exact.matches("//api//secret"));
        assert!(exact.matches("/v1/../api/secret?param=val"));
        assert!(exact.matches("/api/%2e%2e/api/secret#anchor"));
        assert!(!exact.matches("/api/secret/extra"));
    }
}
