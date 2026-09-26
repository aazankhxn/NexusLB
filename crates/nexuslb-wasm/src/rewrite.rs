use crate::filter::{FilterAction, NexusFilter};

pub struct HeaderRewriteFilter {
    name: String,
    add_request_headers: Vec<(String, String)>,
    add_response_headers: Vec<(String, String)>,
    remove_headers: Vec<String>,
}

impl HeaderRewriteFilter {
    pub fn new() -> Self {
        Self {
            name: "header-rewrite-filter".to_string(),
            add_request_headers: Vec::new(),
            add_response_headers: Vec::new(),
            remove_headers: Vec::new(),
        }
    }

    pub fn with_request_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.add_request_headers.push((key.into(), value.into()));
        self
    }

    pub fn with_response_header(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.add_response_headers.push((key.into(), value.into()));
        self
    }

    pub fn with_remove_header(mut self, key: impl Into<String>) -> Self {
        self.remove_headers.push(key.into());
        self
    }
}

impl Default for HeaderRewriteFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusFilter for HeaderRewriteFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn on_request(
        &self,
        _method: &mut String,
        _path: &mut String,
        headers: &mut Vec<(String, String)>,
    ) -> FilterAction {
        headers.retain(|(k, _)| {
            !self
                .remove_headers
                .iter()
                .any(|r| r.eq_ignore_ascii_case(k))
        });
        for (k, v) in &self.add_request_headers {
            headers.push((k.clone(), v.clone()));
        }
        FilterAction::Continue
    }

    fn on_response(&self, _status: &mut u16, headers: &mut Vec<(String, String)>) -> FilterAction {
        headers.retain(|(k, _)| {
            !self
                .remove_headers
                .iter()
                .any(|r| r.eq_ignore_ascii_case(k))
        });
        for (k, v) in &self.add_response_headers {
            headers.push((k.clone(), v.clone()));
        }
        FilterAction::Continue
    }
}
