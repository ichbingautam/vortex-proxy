//! Wasmtime Engine integration for WebAssembly proxy plugins.

use wasmtime::*;

/// Manages the WebAssembly engine, configuration, and module instantiation.
pub struct WasmEngine {
    engine: Engine,
}

impl Default for WasmEngine {
    fn default() -> Self {
        let config = Config::new();
        Self {
            engine: Engine::new(&config).expect("Failed to create Wasmtime Engine"),
        }
    }
}

impl WasmEngine {
    /// Create a new generic WasmEngine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes a simple WebAssembly module by executing 'execute' export.
    pub fn execute_filter(&self, wasm_bytes: &[u8]) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let execute = instance.get_typed_func::<(), i32>(&mut store, "execute")?;
        let result = execute.call(&mut store, ())?;
        Ok(result)
    }
}
