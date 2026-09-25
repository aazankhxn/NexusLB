use std::time::Duration;

pub fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Empty duration string".to_string());
    }

    if let Some(rest) = s.strip_suffix("ms") {
        let val: u64 = rest
            .parse()
            .map_err(|e| format!("Invalid ms number: {}", e))?;
        return Ok(Duration::from_millis(val));
    }
    if let Some(rest) = s.strip_suffix('s') {
        let val: u64 = rest
            .parse()
            .map_err(|e| format!("Invalid seconds number: {}", e))?;
        return Ok(Duration::from_secs(val));
    }
    if let Some(rest) = s.strip_suffix('m') {
        let val: u64 = rest
            .parse()
            .map_err(|e| format!("Invalid minutes number: {}", e))?;
        return Ok(Duration::from_secs(val * 60));
    }
    if let Some(rest) = s.strip_suffix('h') {
        let val: u64 = rest
            .parse()
            .map_err(|e| format!("Invalid hours number: {}", e))?;
        return Ok(Duration::from_secs(val * 3600));
    }

    // Default to seconds if plain number
    let val: u64 = s
        .parse()
        .map_err(|e| format!("Invalid duration number: {}", e))?;
    Ok(Duration::from_secs(val))
}
