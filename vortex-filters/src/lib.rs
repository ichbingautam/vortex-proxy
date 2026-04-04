//! Vortex Wasm Filters
//!
//! Exposes WebAssembly plugin execution via Wasmtime for dynamic proxy filters.

pub mod wasm_engine;
pub mod proxy_wasm;

/// Distributed rate limiting evaluators and redis storage drivers.
pub mod rate_limiter;

/// Initializes the WebAssembly filters runtime.
pub fn filters_init() {
    println!("vortex-filters initialized");
}
