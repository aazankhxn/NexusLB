pub mod duration;
pub mod model;
pub mod validate;

pub use duration::parse_duration;
pub use model::*;
use std::path::Path;
pub use validate::validate_config;

pub fn load_from_str(yaml_str: &str) -> Result<NexusConfig, String> {
    let config: NexusConfig =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parsing error: {}", e))?;

    validate_config(&config)?;
    Ok(config)
}

pub fn load_from_file(path: impl AsRef<Path>) -> Result<NexusConfig, String> {
    let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
        format!(
            "Failed to read config file '{}': {}",
            path.as_ref().display(),
            e
        )
    })?;

    load_from_str(&content)
}
