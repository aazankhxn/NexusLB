use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

pub fn init_observability(log_level: &str, json_format: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    if json_format {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .json()
            .with_target(false)
            .with_filter(filter);

        tracing_subscriber::registry().with(fmt_layer).init();
    } else {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_thread_ids(true)
            .with_filter(filter);

        tracing_subscriber::registry().with(fmt_layer).init();
    }
}
