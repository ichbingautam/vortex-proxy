//! Vortex Proxy core library.

#![allow(missing_docs)]

pub mod ban_manager;
pub mod circuit_breaker;
pub mod connection_pool;
pub mod health_check;
pub mod metrics_ext;
pub mod quic_server;
pub mod server;
pub mod telemetry;
pub mod tls;
