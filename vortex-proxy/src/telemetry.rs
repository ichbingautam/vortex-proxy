//! Telemetry and OpenTelemetry integration for Vortex Proxy.
//!
//! Configures a non-blocking, MPSC-backed batch exporter for W3C trace contexts.

use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{BatchConfig, RandomIdGenerator, Sampler};
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

/// Initializes the global tracing subscriber with an OTLP exporter pipeline.
pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Set W3C Trace Context as the global propagator
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Configure the OTLP exporter (gRPC)
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317"); // Default OpenTelemetry Collector endpoint

    // Configure the batch span processor to use a dedicated background Tokio task
    // effectively acting as an MPSC queue that unblocks the proxy workers.
    let batch_config = BatchConfig::default()
        .with_max_queue_size(8192)
        .with_max_export_batch_size(512);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // Default to 1% sampling for extremely high throughput (100M+ loads)
                // in production this would be configurable.
                .with_sampler(Sampler::TraceIdRatioBased(0.01))
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(Resource::new(vec![opentelemetry::KeyValue::new(
                    "service.name",
                    "vortex-proxy",
                )])),
        )
        .with_batch_config(batch_config)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // Create the tracing layer
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // Create a filter to control log verbosity
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Combine and set as the global default
    Registry::default()
        .with(env_filter)
        .with(telemetry_layer)
        // Also log to stdout for local debugging
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
