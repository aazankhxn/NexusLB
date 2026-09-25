use nexuslb_config::{load_from_str, parse_duration};

#[test]
fn test_duration_parsing() {
    assert_eq!(parse_duration("500ms").unwrap().as_millis(), 500);
    assert_eq!(parse_duration("5s").unwrap().as_secs(), 5);
    assert_eq!(parse_duration("2m").unwrap().as_secs(), 120);
    assert_eq!(parse_duration("1h").unwrap().as_secs(), 3600);
    assert!(parse_duration("invalid").is_err());
}

#[test]
fn test_valid_config() {
    let yaml = r#"
server:
  listen:
    - "0.0.0.0:8080"
  workers: auto
  engine: auto

load_balancer:
  algorithm: adaptive

backends:
  - name: api-1
    address: "10.0.0.1:8080"
    weight: 100
  - name: api-2
    address: "10.0.0.2:8080"
    weight: 100

health_check:
  enabled: true
  interval: 5s
  timeout: 1s
"#;
    let res = load_from_str(yaml);
    assert!(res.is_ok(), "Config should parse cleanly: {:?}", res.err());
}

#[test]
fn test_invalid_config_empty_backends() {
    let yaml = r#"
server:
  listen:
    - "0.0.0.0:8080"
backends: []
"#;
    let res = load_from_str(yaml);
    assert!(res.is_err(), "Must reject empty backends");
}

#[test]
fn test_invalid_config_duplicate_backend_address() {
    let yaml = r#"
server:
  listen:
    - "0.0.0.0:8080"
backends:
  - name: api-1
    address: "10.0.0.1:8080"
  - name: api-2
    address: "10.0.0.1:8080"
"#;
    let res = load_from_str(yaml);
    assert!(res.is_err(), "Must reject duplicate backend addresses");
}
