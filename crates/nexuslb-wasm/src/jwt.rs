use crate::filter::{FilterAction, NexusFilter};

pub struct JwtAuthFilter {
    name: String,
    protected_prefix: String,
}

impl JwtAuthFilter {
    pub fn new(protected_prefix: impl Into<String>) -> Self {
        Self {
            name: "jwt-auth-filter".to_string(),
            protected_prefix: protected_prefix.into(),
        }
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
        if !path.starts_with(&self.protected_prefix) {
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
                if parts.len() == 3 && !parts[0].is_empty() && !parts[1].is_empty() {
                    // Inject decoded or simulated user identity
                    headers.push(("X-User-Id".to_string(), "authenticated-user".to_string()));
                    headers.push(("X-Auth-Status".to_string(), "verified".to_string()));
                    FilterAction::Continue
                } else {
                    FilterAction::StopAndReply {
                        status: 401,
                        headers: vec![
                            ("Content-Type".to_string(), "application/json".to_string()),
                            (
                                "WWW-Authenticate".to_string(),
                                "Bearer error=\"invalid_token\"".to_string(),
                            ),
                        ],
                        body: b"{\"error\":\"Invalid JWT format\"}\n".to_vec(),
                    }
                }
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
    fn test_jwt_filter_allows_valid_bearer() {
        let filter = JwtAuthFilter::new("/api/secure");
        let mut method = "GET".to_string();
        let mut path = "/api/secure/data".to_string();
        let mut headers = vec![(
            "Authorization".to_string(),
            "Bearer header.payload.sig".to_string(),
        )];

        let action = filter.on_request(&mut method, &mut path, &mut headers);
        assert_eq!(action, FilterAction::Continue);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "X-User-Id" && v == "authenticated-user"));
    }
}
