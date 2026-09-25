use crate::pool::PoolGroup;
use crate::route::Route;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    pools: HashMap<String, PoolGroup>,
    default_pool: Option<PoolGroup>,
}

impl Router {
    pub fn new(
        mut routes: Vec<Route>,
        pools: HashMap<String, PoolGroup>,
        default_pool_name: Option<&str>,
    ) -> Self {
        // Sort routes by priority descending (highest priority evaluated first)
        routes.sort_by_key(|a| std::cmp::Reverse(a.priority));

        let default_pool = default_pool_name.and_then(|name| pools.get(name).cloned());

        Self {
            routes,
            pools,
            default_pool,
        }
    }

    #[inline(always)]
    pub fn route(
        &self,
        host: Option<&str>,
        path: &str,
        method: Option<&str>,
        headers: Option<&[(String, String)]>,
        sni: Option<&str>,
    ) -> Option<(&Route, &PoolGroup)> {
        for route in &self.routes {
            if route.matches(host, path, method, headers, sni) {
                if let Some(pool) = self.pools.get(&route.pool_name) {
                    return Some((route, pool));
                }
            }
        }

        None
    }

    pub fn default_pool(&self) -> Option<&PoolGroup> {
        self.default_pool.as_ref()
    }

    pub fn pools(&self) -> &HashMap<String, PoolGroup> {
        &self.pools
    }

    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}
