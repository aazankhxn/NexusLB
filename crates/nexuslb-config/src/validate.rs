use crate::duration::parse_duration;
use crate::model::NexusConfig;
use std::collections::HashSet;
use std::net::SocketAddr;

pub fn validate_config(config: &NexusConfig) -> Result<(), String> {
    // 1. Validate server listen addresses
    if config.server.listen.is_empty() {
        return Err(
            "Server must specify at least one listen address in 'server.listen'".to_string(),
        );
    }
    for listen in &config.server.listen {
        listen
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid listen address '{}': {}", listen, e))?;
    }

    // 2. Validate backends
    if config.backends.is_empty() {
        return Err("Configuration must define at least one backend in 'backends'".to_string());
    }

    let mut backend_names = HashSet::new();
    let mut backend_addrs = HashSet::new();

    for b in &config.backends {
        if b.name.trim().is_empty() {
            return Err("Backend name cannot be empty".to_string());
        }
        if !backend_names.insert(&b.name) {
            return Err(format!("Duplicate backend name '{}'", b.name));
        }

        b.address.parse::<SocketAddr>().map_err(|e| {
            format!(
                "Invalid backend address '{}' for backend '{}': {}",
                b.address, b.name, e
            )
        })?;

        if !backend_addrs.insert(&b.address) {
            return Err(format!("Duplicate backend address '{}'", b.address));
        }

        if b.weight == 0 {
            return Err(format!("Backend '{}' weight must be >= 1", b.name));
        }
    }

    // 3. Validate algorithm
    let valid_algorithms = [
        "round_robin",
        "weighted_round_robin",
        "least_connections",
        "random",
        "ip_hash",
        "consistent_hash",
        "power_of_two_choices",
        "least_latency",
        "ewma_latency",
        "adaptive",
    ];
    if !valid_algorithms.contains(&config.load_balancer.algorithm.as_str()) {
        return Err(format!(
            "Unknown load balancer algorithm '{}'. Supported: {:?}",
            config.load_balancer.algorithm, valid_algorithms
        ));
    }

    // 4. Validate health check durations
    if config.health_check.enabled {
        parse_duration(&config.health_check.interval)
            .map_err(|e| format!("Invalid health_check.interval: {}", e))?;
        parse_duration(&config.health_check.timeout)
            .map_err(|e| format!("Invalid health_check.timeout: {}", e))?;
        if config.health_check.healthy_threshold == 0 {
            return Err("health_check.healthy_threshold must be >= 1".to_string());
        }
        if config.health_check.unhealthy_threshold == 0 {
            return Err("health_check.unhealthy_threshold must be >= 1".to_string());
        }
    }

    // 5. Validate circuit breaker
    if config.circuit_breaker.enabled {
        parse_duration(&config.circuit_breaker.cool_down)
            .map_err(|e| format!("Invalid circuit_breaker.cool_down: {}", e))?;
        if config.circuit_breaker.failure_threshold == 0 {
            return Err("circuit_breaker.failure_threshold must be >= 1".to_string());
        }
    }

    // 6. Validate metrics address
    if config.metrics.enabled {
        config.metrics.address.parse::<SocketAddr>().map_err(|e| {
            format!(
                "Invalid metrics address '{}': {}",
                config.metrics.address, e
            )
        })?;
    }

    // 7. Validate admin address and security
    if config.admin.enabled {
        config
            .admin
            .address
            .parse::<SocketAddr>()
            .map_err(|e| format!("Invalid admin address '{}': {}", config.admin.address, e))?;

        if config.admin.authentication.required {
            match &config.admin.token {
                None => return Err("Admin API is enabled with authentication required, but no admin.token is configured. Set a secure token or set admin.authentication.required: false (NOT recommended).".to_string()),
                Some(t) if t.trim().is_empty() => return Err("Admin API token cannot be empty when authentication is required.".to_string()),
                Some(t) if t == "nexuslb-admin-secret-change-in-production" => {
                    return Err("Admin API is using insecure default placeholder token. Set a secure secret token in production.".to_string());
                }
                _ => {}
            }
        }
    }

    Ok(())
}
