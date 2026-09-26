use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterAction {
    /// Proceed to the next filter in the pipeline or upstream backend
    Continue,
    /// Short-circuit the request and return an immediate response to client
    StopAndReply {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    },
    /// Abruptly terminate the connection
    Drop,
}

#[async_trait]
pub trait NexusFilter: Send + Sync {
    /// Friendly identifier of the filter plugin
    fn name(&self) -> &str;

    /// Invoked before request is forwarded to backend
    fn on_request(
        &self,
        method: &mut String,
        path: &mut String,
        headers: &mut Vec<(String, String)>,
    ) -> FilterAction {
        let _ = (method, path, headers);
        FilterAction::Continue
    }

    /// Invoked before response is returned to client
    fn on_response(&self, status: &mut u16, headers: &mut Vec<(String, String)>) -> FilterAction {
        let _ = (status, headers);
        FilterAction::Continue
    }
}
