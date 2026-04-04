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
    pub fn execute_filter(
        &self,
        wasm_bytes: &[u8],
    ) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &module, &[])?;
        let execute = instance.get_typed_func::<(), i32>(&mut store, "execute")?;
        let result = execute.call(&mut store, ())?;
        Ok(result)
    }

    /// Prepares and executes a Proxy-WASM compliant module.
    pub fn execute_proxy_wasm<T: crate::proxy_wasm::host::HostEnvironment + 'static>(
        &self,
        wasm_bytes: &[u8],
        env: T,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        
        let mut store = Store::new(&self.engine, env);
        let mut linker = Linker::new(&self.engine);
        
        // Register Proxy-Wasm host bindings onto the Linker
        crate::proxy_wasm::host::register_proxy_wasm_abi(&mut linker)?;
        
        // Let WASI handle undefined imports gracefully or stub them out.
        // For a full implementation, WASI standard imports must be provided.
        // wasmtime_wasi::add_to_linker(&mut linker, ...);
        
        let instance = linker.instantiate(&mut store, &module)?;
        
        // Proxy-WASM ABI uses _start as initialization, and then lifecycle handlers like
        // proxy_on_configure, proxy_on_request_headers, etc.
        // Here we stub out a call to proxy_on_request_headers which most plugins expect.
        if let Ok(on_request_headers) = instance.get_typed_func::<(i32, i32, i32), i32>(&mut store, "proxy_on_request_headers") {
            // fake root_context_id=1, plugin_context_id=2, stream_id=3
            let _action = on_request_headers.call(&mut store, (1, 2, 3))?;
             // Action is returned (Continue or Pause)
        }

        Ok(())
    }
}
