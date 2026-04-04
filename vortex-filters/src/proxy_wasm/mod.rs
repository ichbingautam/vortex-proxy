//! Proxy-Wasm ABI host implementation for Vortex.
//!
//! This module provides the host environment necessary to execute
//! WebAssembly plugins compiled against the Proxy-Wasm specification (e.g. Envoy plugins).
//! See: https://github.com/proxy-wasm/spec

pub mod host;
pub mod types;

// Re-export core traits
pub use host::HostEnvironment;
pub use types::{Action, WasmResult};
