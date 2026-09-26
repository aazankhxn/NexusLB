use crate::filter::{FilterAction, NexusFilter};
use std::sync::Arc;

pub struct FilterChain {
    filters: Vec<Arc<dyn NexusFilter>>,
}

impl FilterChain {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add_filter(&mut self, filter: Arc<dyn NexusFilter>) {
        self.filters.push(filter);
    }

    pub fn execute_request(
        &self,
        method: &mut String,
        path: &mut String,
        headers: &mut Vec<(String, String)>,
    ) -> FilterAction {
        for filter in &self.filters {
            match filter.on_request(method, path, headers) {
                FilterAction::Continue => continue,
                action => return action,
            }
        }
        FilterAction::Continue
    }

    pub fn execute_response(
        &self,
        status: &mut u16,
        headers: &mut Vec<(String, String)>,
    ) -> FilterAction {
        for filter in &self.filters {
            match filter.on_response(status, headers) {
                FilterAction::Continue => continue,
                action => return action,
            }
        }
        FilterAction::Continue
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        Self::new()
    }
}
