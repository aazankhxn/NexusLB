use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathMatch {
    Exact(String),
    Prefix(String),
    Any,
}

impl PathMatch {
    #[inline(always)]
    pub fn matches(&self, path: &str) -> bool {
        match self {
            PathMatch::Exact(expected) => path == expected,
            PathMatch::Prefix(prefix) => path.starts_with(prefix),
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
}
