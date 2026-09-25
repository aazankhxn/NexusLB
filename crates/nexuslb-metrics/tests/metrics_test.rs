use nexuslb_metrics::GlobalMetrics;
use std::time::Duration;

#[test]
fn test_metrics_aggregation_and_prometheus() {
    let global = GlobalMetrics::new(4);

    let w0 = global.worker(0);
    let w1 = global.worker(1);

    w0.inc_requests();
    w0.inc_requests();
    w0.inc_connections();
    w0.record_latency(Duration::from_millis(5));

    w1.inc_requests();
    w1.inc_connections();
    w1.record_latency(Duration::from_millis(15));

    let agg = global.aggregate();
    assert_eq!(agg.requests_total, 3);
    assert_eq!(agg.connections_total, 2);
    assert_eq!(agg.latency_count, 2);

    let prom = global.render_prometheus();
    assert!(prom.contains("nexuslb_requests_total 3"));
    assert!(prom.contains("nexuslb_connections_total 2"));
    assert!(prom.contains("nexuslb_request_duration_seconds_bucket"));
}
