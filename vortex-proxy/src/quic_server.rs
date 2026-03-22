//! HTTP/3 QUIC Server configuration using Quinn.
//!
//! Provides the UDP listener and QUIC protocol negotiation.

use crate::tls::{load_certs, load_private_key};
use quinn::crypto::rustls::QuicServerConfig;
use quinn::{Endpoint, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{error, info};
use vortex_core::domain::routing::SharedRoutingTable;

/// Starts a QUIC listener on the specified UDP address.
pub async fn start_quic_server(
    addr: SocketAddr,
    cert_path: &str,
    key_path: &str,
    _routing_table: SharedRoutingTable,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let mut rustls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    // Specify ALPN for HTTP/3
    rustls_config.alpn_protocols = vec![b"h3".to_vec()];

    // Create the quinn-specific crypto config using the `quinn::crypto::rustls::QuicServerConfig` wrapper
    let crypto_config = QuicServerConfig::try_from(rustls_config)?;

    let server_config = ServerConfig::with_crypto(Arc::new(crypto_config));
    let endpoint = Endpoint::server(server_config, addr)?;

    info!("QUIC (HTTP/3) listening on {}", addr);

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            match conn.await {
                Ok(connection) => {
                    info!(
                        "QUIC connection established from {}",
                        connection.remote_address()
                    );
                    // In a production implementation, we would spawn an h3::server handler here
                    // to extract the HTTP semantics from the QUIC stream and route it.
                }
                Err(e) => {
                    error!("QUIC connection failed: {}", e);
                }
            }
        });
    }

    Ok(())
}
