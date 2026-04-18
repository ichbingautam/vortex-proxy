//! Prometheus Metrics Integration for Vortex Proxy.
//!
//! Configures a standalone HTTP listener for scraping high-resolution metrics.

use metrics_exporter_prometheus::PrometheusBuilder;
use std::net::SocketAddr;

/// Initializes the Prometheus metrics exporter on the specified port.
/// By default, we use port 9091 to avoid conflicting with the proxy or admin APIs.
pub fn init_metrics_exporter(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Build the Prometheus exporter and spawn its internal HTTP server
    let builder = PrometheusBuilder::new().with_http_listener(addr);

    // Optionally setup buckets for histograms (e.g. request duration)
    let builder = builder.set_buckets_for_metric(
        metrics_exporter_prometheus::Matcher::Full("vortex_request_duration_seconds".to_string()),
        &[
            0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
        ],
    )?;

    // AI Gateway specific metrics: Token usage distribution
    let builder = builder.set_buckets_for_metric(
        metrics_exporter_prometheus::Matcher::Full("vortex_ai_token_usage".to_string()),
        &[
            10.0, 50.0, 100.0, 500.0, 1000.0, 2048.0, 4096.0, 8192.0, 16384.0, 32768.0,
        ],
    )?;

    builder.install()?;

    tracing::info!("Prometheus metrics exporter started on http://{}", addr);

    Ok(())
}
