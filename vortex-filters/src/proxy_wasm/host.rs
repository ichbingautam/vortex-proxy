//! Proxy-Wasm host environment instantiation.
use super::types::WasmResult;
use wasmtime::*;

/// Represents the State that the Host Functions will have access to
/// during a webassembly module's execution.
pub trait HostEnvironment: Send + Sync {
    /// proxy_log host function
    fn log(&self, level: i32, message: &str) -> WasmResult;

    /// proxy_get_header_map_value host function
    fn get_header_map_value(&self, map_type: i32, key: &str) -> Option<String>;

    // Add additional Proxy-WASM ABI functions here (get_property, send_local_response, etc)
}

/// Registers the official Proxy-WASM ABI host functions into a wasmtime Linker.
///
/// `T` must contain a reference or the actual `HostEnvironment` traits implementation.
pub fn register_proxy_wasm_abi<T>(
    linker: &mut Linker<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    T: HostEnvironment + 'static,
{
    // The Proxy-WASM ABI typically exists under the "env" module in most Envoy compliant setups.

    // proxy_log
    linker.func_wrap(
        "env",
        "proxy_log",
        |mut caller: Caller<'_, T>, level: i32, message_data: i32, message_size: i32| -> i32 {
            // Extract string from memory (simplified for boilerplate)
            let mem = match caller.get_export("memory") {
                Some(Extern::Memory(m)) => m,
                _ => return WasmResult::InternalFailure as i32,
            };

            let mut buffer = vec![0u8; message_size as usize];
            if mem
                .read(&caller, message_data as usize, &mut buffer)
                .is_err()
            {
                return WasmResult::InvalidMemoryAccess as i32;
            }

            let message = match std::str::from_utf8(&buffer) {
                Ok(s) => s,
                Err(_) => return WasmResult::ParseFailure as i32,
            };

            // Dispatch to HostEnvironment implementation
            let env = caller.data();
            env.log(level, message) as i32
        },
    )?;

    // proxy_get_header_map_value
    linker.func_wrap(
        "env",
        "proxy_get_header_map_value",
        |mut caller: Caller<'_, T>,
         map_type: i32,
         key_data: i32,
         key_size: i32,
         _return_value_data: i32,
         _return_value_size: i32|
         -> i32 {
            let mem = match caller.get_export("memory") {
                Some(Extern::Memory(m)) => m,
                _ => return WasmResult::InternalFailure as i32,
            };

            let mut key_buffer = vec![0u8; key_size as usize];
            if mem
                .read(&caller, key_data as usize, &mut key_buffer)
                .is_err()
            {
                return WasmResult::InvalidMemoryAccess as i32;
            }

            let key_str = match std::str::from_utf8(&key_buffer) {
                Ok(s) => s,
                Err(_) => return WasmResult::ParseFailure as i32,
            };

            let env = caller.data();
            if let Some(_val) = env.get_header_map_value(map_type, key_str) {
                // Write back logic to guest memory is needed here (allocating on guest)
                // We'd typically call proxy_on_memory_allocate. For brevity, skipped in this stub.
                println!("Proxy-wasm stub: intercepted header read for {}", key_str);
                WasmResult::Ok as i32
            } else {
                WasmResult::NotFound as i32
            }
        },
    )?;

    // proxy_continue_stream
    linker.func_wrap(
        "env",
        "proxy_continue_stream",
        |_caller: Caller<'_, T>, _stream_type: i32| -> i32 {
            // Unpause the HTTP stream
            WasmResult::Ok as i32
        },
    )?;

    Ok(())
}
