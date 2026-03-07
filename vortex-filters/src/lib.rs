//! Vortex Wasm Filters
//!
//! Exposes WebAssembly plugin execution via Wasmtime for dynamic proxy filters.

pub mod wasm_engine;

/// Initializes the WebAssembly filters runtime.
pub fn filters_init() {
    println!("vortex-filters initialized");
}
