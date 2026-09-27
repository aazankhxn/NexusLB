use base64::engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use ring::hmac;
use serde::Deserialize;

use crate::filter::{FilterAction, NexusFilter};

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    sub: Option<String>,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    exp: Option<u64>,
    #[serde(default)]
    nbf: Option<u64>,
}

pub struct JwtAuthFilter {
    name: String,
    protected_prefix: String,
    secret: Option<Vec<u8>>,
}

impl JwtAuthFilter {
    pub fn new(protected_prefix: impl Into<String>) -> Self {
        Self {
            name: "jwt-auth-filter".to_string(),
            protected_prefix: protected_prefix.into(),
            secret: None,
        }
    }

    pub fn with_secret(mut self, secret: impl Into<Vec<u8>>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    fn decode_b64(input: &str) -> Option<Vec<u8>> {
        let trimmed = input.trim_end_matches('=');
        URL_SAFE_NO_PAD
            .decode(trimmed)
            .ok()
            .or_else(|| URL_SAFE.decode(input).ok())
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
                if let (Some(h1), Some(h2)) = (Self::hex_val(bytes[i + 1]), Self::hex_val(bytes[i + 2])) {
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

    fn decode_path(raw: &str) -> String {
        let mut current = raw.to_string();
        for _ in 0..2 {
            let next = Self::percent_decode(&current);
            if next == current {
                break;
            }
            current = next;
        }
        current
    }

    /// Canonicalize HTTP request paths by resolving '.', '..', duplicate slashes,
    /// stripping query strings / fragments, and decoding percent-encoded traversal sequences (CWE-22)
    pub fn normalize_path(raw_path: &str) -> String {
        let path_only = raw_path.split('?').next().unwrap_or(raw_path);
        let path_only = path_only.split('#').next().unwrap_or(path_only);
        let decoded = Self::decode_path(path_only);

        let mut segments = Vec::new();
        for seg in decoded.split('/') {
            let s = seg.trim();
            match s {
                "" | "." => continue,
                ".." => {
                    segments.pop();
                }
                valid => segments.push(valid),
            }
        }

        let mut normalized = String::with_capacity(decoded.len() + 1);
        normalized.push('/');
        normalized.push_str(&segments.join("/"));
        normalized
    }

    fn matches_prefix(&self, normalized_path: &str) -> bool {
        let prefix = self.protected_prefix.trim_end_matches('/');
        if prefix.is_empty() {
            return true;
        }
        if normalized_path == prefix {
            return true;
        }
        if let Some(remainder) = normalized_path.strip_prefix(prefix) {
            let next_char = remainder.chars().next();
            if next_char == Some('/') || next_char.is_none() {
                return true;
            }
        }
        false
    }
}

impl NexusFilter for JwtAuthFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_request(
        &self,
        _method: &mut String,
        path: &mut String,
        headers: &mut Vec<(String, String)>,
    ) -> FilterAction {
        // SECURITY: Always strip internal identity headers from ALL incoming requests
        // to prevent clients from spoofing authenticated identity on non-protected paths.
        // These headers are injected by this filter only after successful JWT verification.
        headers.retain(|(k, _)| {
            !k.eq_ignore_ascii_case("x-auth-subject")
                && !k.eq_ignore_ascii_case("x-user-id")
                && !k.eq_ignore_ascii_case("x-auth-status")
        });

        let normalized = Self::normalize_path(path);
        if !self.matches_prefix(&normalized) {
            return FilterAction::Continue;
        }

        // Search for Authorization: Bearer <token>
        let auth_hdr = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.as_str());

        match auth_hdr {
            Some(val) if val.starts_with("Bearer ") => {
                let token = val.trim_start_matches("Bearer ").trim();
                // Validate JWT 3-part structure (header.payload.signature)
                let parts: Vec<&str> = token.split('.').collect();
                if parts.len() != 3 || parts[0].is_empty() || parts[1].is_empty() {
                    return FilterAction::StopAndReply {
                        status: 401,
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string()),
                            (
                                "WWW-Authenticate".to_string(),
                                "Bearer error=\"invalid_token\"".to_string(),
                            ),
                        ],
                        body: b"{\"error\":\"Invalid JWT format\"}\n".to_vec(),
                    };
                }

                // SECURITY: Validate JWT header algorithm to block algorithm confusion attacks.
                // Fail-closed defense: reject malformed base64 or JSON headers immediately.
                let header_bytes = match Self::decode_b64(parts[0]) {
                    Some(b) => b,
                    None => {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"Malformed JWT header encoding\"}\n".to_vec(),
                        };
                    }
                };

                #[derive(serde::Deserialize)]
                struct JwtHeader {
                    #[serde(default)]
                    alg: Option<String>,
                }

                let header = match serde_json::from_slice::<JwtHeader>(&header_bytes) {
                    Ok(h) => h,
                    Err(_) => {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"Malformed JWT header JSON\"}\n".to_vec(),
                        };
                    }
                };

                match header.alg.as_deref() {
                    Some("HS256") => {} // Only accepted algorithm
                    Some("none") | None => {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"JWT algorithm 'none' is not permitted\"}\n"
                                .to_vec(),
                        };
                    }
                    Some(alg) => {
                        let msg = format!(
                            "{{\"error\":\"Unsupported JWT algorithm: {}. Only HS256 is accepted.\"}}\n",
                            alg.chars().take(16).collect::<String>() // Truncate to prevent log injection
                        );
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: msg.into_bytes(),
                        };
                    }
                }

                // SECURITY: Cryptographically verify the HMAC signature.
                // If no secret is configured, reject ALL tokens on protected paths.
                // This prevents the classic "no-verification" bypass when secret is unset.
                match self.secret {
                    Some(ref secret) => {
                        let sig_bytes = match Self::decode_b64(parts[2]) {
                            Some(b) => b,
                            None => {
                                return FilterAction::StopAndReply {
                                    status: 401,
                                    headers: vec![
                                        ("Content-Type".to_string(), "application/json".to_string()),
                                        (
                                            "WWW-Authenticate".to_string(),
                                            "Bearer error=\"invalid_token\"".to_string(),
                                        ),
                                    ],
                                    body: b"{\"error\":\"Malformed JWT signature encoding\"}\n"
                                        .to_vec(),
                                };
                            }
                        };

                        let signed_data = format!("{}.{}", parts[0], parts[1]);
                        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
                        if hmac::verify(&key, signed_data.as_bytes(), &sig_bytes).is_err() {
                            return FilterAction::StopAndReply {
                                status: 401,
                                headers: vec![
                                    ("Content-Type".to_string(), "application/json".to_string()),
                                    (
                                        "WWW-Authenticate".to_string(),
                                        "Bearer error=\"invalid_token\"".to_string(),
                                    ),
                                ],
                                body: b"{\"error\":\"Invalid JWT signature\"}\n".to_vec(),
                            };
                        }
                    }
                    None => {
                        // Fail-closed defense: If no secret is configured on a protected path,
                        // reject all incoming tokens with 401 rather than allowing unverified access.
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\", error_description=\"JWT verification secret not configured\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"JWT verification secret not configured on load balancer\"}\n".to_vec(),
                        };
                    }
                }

                // Parse and validate payload claims (reject malformed base64 or invalid JSON)
                let payload_bytes = match Self::decode_b64(parts[1]) {
                    Some(b) => b,
                    None => {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"Malformed JWT payload encoding\"}\n".to_vec(),
                        };
                    }
                };

                let claims = match serde_json::from_slice::<JwtClaims>(&payload_bytes) {
                    Ok(c) => c,
                    Err(_) => {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                ("Content-Type".to_string(), "application/json".to_string()),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"invalid_token\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"Malformed JWT payload claims\"}\n".to_vec(),
                        };
                    }
                };

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                if let Some(exp) = claims.exp {
                    if now > exp {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                (
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                ),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"token_expired\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"JWT token expired\"}\n".to_vec(),
                        };
                    }
                }

                if let Some(nbf) = claims.nbf {
                    if now < nbf {
                        return FilterAction::StopAndReply {
                            status: 401,
                            headers: vec![
                                (
                                    "Content-Type".to_string(),
                                    "application/json".to_string(),
                                ),
                                (
                                    "WWW-Authenticate".to_string(),
                                    "Bearer error=\"token_not_yet_valid\"".to_string(),
                                ),
                            ],
                            body: b"{\"error\":\"JWT token not yet valid\"}\n".to_vec(),
                        };
                    }
                }

                if let Some(sub) = claims.sub.or(claims.user_id) {
                    // Sanitize against CRLF injection in downstream identity headers
                    let safe_sub = sub
                        .chars()
                        .filter(|&c| c >= ' ' && c != '\x7f')
                        .collect::<String>();
                    headers.push(("X-Auth-Subject".to_string(), safe_sub));
                }

                // Inject authenticated identity into downstream headers
                headers.push(("X-User-Id".to_string(), "authenticated-user".to_string()));
                headers.push(("X-Auth-Status".to_string(), "verified".to_string()));
                FilterAction::Continue
            }
            _ => FilterAction::StopAndReply {
                status: 401,
                headers: vec![
                    ("Content-Type".to_string(), "application/json".to_string()),
                    ("WWW-Authenticate".to_string(), "Bearer".to_string()),
                ],
                body: b"{\"error\":\"Missing or malformed Authorization header\"}\n".to_vec(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_filter_blocks_unauthorized() {
        let filter = JwtAuthFilter::new("/api/secure");
        let mut method = "GET".to_string();
        let mut path = "/api/secure/data".to_string();
        let mut headers = vec![];

        let action = filter.on_request(&mut method, &mut path, &mut headers);
        match action {
            FilterAction::StopAndReply { status, .. } => assert_eq!(status, 401),
            _ => panic!("Expected 401 Unauthorized"),
        }
    }

    #[test]
    fn test_jwt_filter_rejects_when_secret_unset() {
        let filter = JwtAuthFilter::new("/api/secure"); // No secret configured
        let mut method = "GET".to_string();
        let mut path = "/api/secure/data".to_string();
        let mut headers = vec![("Authorization".to_string(), "Bearer header.payload.signature".to_string())];

        let action = filter.on_request(&mut method, &mut path, &mut headers);
        match action {
            FilterAction::StopAndReply { status, .. } => assert_eq!(status, 401),
            _ => panic!("Expected 401 fail-closed when secret is missing"),
        }
    }

    #[test]
    fn test_jwt_filter_allows_valid_bearer() {
        let secret = b"unit-test-secret-key-12345";
        let filter = JwtAuthFilter::new("/api/secure").with_secret(&secret[..]);
        let mut method = "GET".to_string();
        let mut path = "/api/secure/data".to_string();

        let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let payload = URL_SAFE_NO_PAD.encode(b"{\"sub\":\"alice\"}");
        let unsigned = format!("{}.{}", header, payload);
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let tag = hmac::sign(&key, unsigned.as_bytes());
        let sig = URL_SAFE_NO_PAD.encode(tag.as_ref());
        let valid_token = format!("{}.{}", unsigned, sig);

        let mut headers = vec![(
            "Authorization".to_string(),
            format!("Bearer {}", valid_token),
        )];

        let action = filter.on_request(&mut method, &mut path, &mut headers);
        assert_eq!(action, FilterAction::Continue);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-User-Id" && v == "authenticated-user"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-Auth-Subject" && v == "alice"));
    }

    #[test]
    fn test_jwt_filter_with_hmac_signature_verification() {
        let secret = b"my-super-secret-key-123456";
        let filter = JwtAuthFilter::new("/api/secure").with_secret(&secret[..]);

        let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"; // {"alg":"HS256","typ":"JWT"}
        let payload = "eyJzdWIiOiJ1c2VyLTQyIiwiaWF0IjoxNTE2MjM5MDIyfQ"; // {"sub":"user-42","iat":1516239022}
        let unsigned = format!("{}.{}", header, payload);

        // Sign with ring
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let tag = hmac::sign(&key, unsigned.as_bytes());
        let sig = URL_SAFE_NO_PAD.encode(tag.as_ref());
        let valid_token = format!("{}.{}", unsigned, sig);

        // 1. Valid token passes and extracts sub
        let mut method = "GET".to_string();
        let mut path = "/api/secure/profile".to_string();
        let mut headers = vec![(
            "Authorization".to_string(),
            format!("Bearer {}", valid_token),
        )];
        let action = filter.on_request(&mut method, &mut path, &mut headers);
        assert_eq!(action, FilterAction::Continue);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-Auth-Subject" && v == "user-42"));
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-User-Id" && v == "authenticated-user"));

        // 2. Tampered token fails signature verification
        let tampered_token = format!("{}.tampered_signature", unsigned);
        let mut headers_tampered = vec![(
            "Authorization".to_string(),
            format!("Bearer {}", tampered_token),
        )];
        let action_tampered = filter.on_request(&mut method, &mut path, &mut headers_tampered);
        match action_tampered {
            FilterAction::StopAndReply { status, .. } => assert_eq!(status, 401),
            _ => panic!("Expected 401 for tampered token"),
        }
    }

    #[test]
    fn test_jwt_filter_path_normalization_traversal_defense() {
        let filter = JwtAuthFilter::new("/api/secure");

        // Traversal attempts must NOT bypass authentication (including percent-encoded variants)
        let traversal_paths = [
            "/public/../api/secure/admin",
            "//api/secure/admin",
            "/api/./secure/admin",
            "/api/secure/../secure/admin?param=1",
            "/public/%2e%2e/api/secure/admin",
            "/public/%2E%2E/api/secure/admin",
            "/public/..%2fapi/secure/admin",
            "/public/%252e%252e/api/secure/admin",
            "/api/secure/%2e%2e/../api/secure/data",
        ];

        for p in traversal_paths {
            let mut method = "GET".to_string();
            let mut path = p.to_string();
            let mut headers = vec![];
            let action = filter.on_request(&mut method, &mut path, &mut headers);
            match action {
                FilterAction::StopAndReply { status, .. } => assert_eq!(
                    status, 401,
                    "Path traversal bypass '{}' should return 401",
                    p
                ),
                _ => panic!("Path traversal bypass '{}' was not blocked!", p),
            }
        }
    }

    #[test]
    fn test_jwt_filter_malformed_payload_and_crlf_defense() {
        let secret = b"unit-test-crlf-key-12345";
        let filter = JwtAuthFilter::new("/api/secure").with_secret(&secret[..]);

        // 1. Malformed base64 payload must be rejected
        let malformed_b64 = "eyJhbGciOiJIUzI1NiJ9.!!!not-base64!!!.sig";
        let mut method = "GET".to_string();
        let mut path = "/api/secure/data".to_string();
        let mut headers = vec![("Authorization".to_string(), format!("Bearer {}", malformed_b64))];
        let action = filter.on_request(&mut method, &mut path, &mut headers);
        match action {
            FilterAction::StopAndReply { status, .. } => assert_eq!(status, 401),
            _ => panic!("Malformed base64 payload must be rejected with 401"),
        }

        // 2. Non-JSON payload must be rejected (with valid signature so it reaches payload parsing)
        let header = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let non_json_b64 = URL_SAFE_NO_PAD.encode(b"this is plain text not json");
        let unsigned = format!("{}.{}", header, non_json_b64);
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let tag = hmac::sign(&key, unsigned.as_bytes());
        let sig = URL_SAFE_NO_PAD.encode(tag.as_ref());
        let non_json_token = format!("{}.{}", unsigned, sig);

        let mut headers2 = vec![("Authorization".to_string(), format!("Bearer {}", non_json_token))];
        let action2 = filter.on_request(&mut method, &mut path, &mut headers2);
        match action2 {
            FilterAction::StopAndReply { status, .. } => assert_eq!(status, 401),
            _ => panic!("Non-JSON payload must be rejected with 401"),
        }

        // 3. CRLF in claims subject must be stripped to prevent HTTP response/header splitting
        let crlf_payload = r#"{"sub":"admin\r\nX-Injected: attack"}"#;
        let crlf_b64 = URL_SAFE_NO_PAD.encode(crlf_payload.as_bytes());
        let unsigned_crlf = format!("{}.{}", header, crlf_b64);
        let tag_crlf = hmac::sign(&key, unsigned_crlf.as_bytes());
        let sig_crlf = URL_SAFE_NO_PAD.encode(tag_crlf.as_ref());
        let crlf_token = format!("{}.{}", unsigned_crlf, sig_crlf);

        let mut headers3 = vec![("Authorization".to_string(), format!("Bearer {}", crlf_token))];
        let action3 = filter.on_request(&mut method, &mut path, &mut headers3);
        assert_eq!(action3, FilterAction::Continue);
        for (k, v) in headers3 {
            assert!(!v.contains('\r') && !v.contains('\n'), "Header {} contains CRLF: {}", k, v);
        }
    }
}

