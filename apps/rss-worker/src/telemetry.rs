use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry as TracingRegistry};

/// Initialize telemetry with tracing and metrics
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = TracingRegistry::default().with(env_filter);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(true);

    let subscriber = subscriber.with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
