//! TLS termination and configuration logic for Vortex.
//!
//! This module handles loading certificates and private keys
//! into a `rustls::ServerConfig`, and providing an acceptor
//! for incoming secure connections.

use pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// Loads certificates from a PEM file.
pub fn load_certs<P: AsRef<Path>>(
    cert_path: P,
) -> Result<Vec<CertificateDer<'static>>, Box<dyn std::error::Error + Send + Sync>> {
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs = rustls_pemfile::certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;
    Ok(certs)
}

/// Loads the first valid PKCS8 private key from a PEM file.
pub fn load_private_key<P: AsRef<Path>>(
    key_path: P,
) -> Result<PrivateKeyDer<'static>, Box<dyn std::error::Error + Send + Sync>> {
    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .map(|res| res.map(PrivateKeyDer::Pkcs8))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(keys.remove(0))
}

/// Loads a TLS `ServerConfig` from the given certificate and key paths (for HTTP/1.1 and H2).
pub fn load_tls_config<P: AsRef<Path>>(
    cert_path: P,
    key_path: P,
) -> Result<Arc<ServerConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(Arc::new(config))
}
