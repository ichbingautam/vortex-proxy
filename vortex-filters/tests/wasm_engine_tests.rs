//! Integration tests for executing WebAssembly (Wasm) filter plugins natively.

use vortex_filters::wasm_engine::WasmEngine;

#[test]
fn test_execute_wat_filter() {
    let engine = WasmEngine::new();

    // A simple WebAssembly Text format representing an immediate evaluation
    // that returns an i32 value (e.g., simulating a filter ACCEPT code 200).
    let wat = r#"
        (module
            (func (export "execute") (result i32)
                i32.const 200
            )
        )
    "#;

    let result = engine
        .execute_filter(wat.as_bytes())
        .expect("Failed to execute WASM module");
    assert_eq!(result, 200);
}

// A simple stub environment for testing
struct TestHostEnv;
impl vortex_filters::proxy_wasm::HostEnvironment for TestHostEnv {
    fn log(&self, _level: i32, message: &str) -> vortex_filters::proxy_wasm::WasmResult {
        println!("WASM Log: {}", message);
        vortex_filters::proxy_wasm::WasmResult::Ok
    }

    fn get_header_map_value(&self, _map_type: i32, _key: &str) -> Option<String> {
        Some("integration_test_value".to_string())
    }
}

#[test]
fn test_execute_proxy_wasm_abi() {
    let engine = WasmEngine::new();

    // A simple WebAssembly Text format simulating a Proxy-WASM plugin.
    // It imports the `proxy_log` host function and exports `proxy_on_request_headers`
    let wat = r#"
        (module
            (import "env" "proxy_log" (func $proxy_log (param i32 i32 i32) (result i32)))
            
            ;; memory
            (memory (export "memory") 1)
            
            ;; store the string "hello from wasm" at offset 0
            (data (i32.const 0) "hello from wasm")
            
            (func (export "proxy_on_request_headers") (param i32 i32 i32) (result i32)
                ;; Call proxy_log(level=2, data_ptr=0, data_size=15)
                (call $proxy_log (i32.const 2) (i32.const 0) (i32.const 15))
                drop ;; drop the result of proxy_log
                
                ;; Return 0 (Continue)
                i32.const 0
            )
        )
    "#;

    // execution shouldn't fail
    engine
        .execute_proxy_wasm(wat.as_bytes(), TestHostEnv)
        .expect("Failed to execute Proxy-WASM plugin");
}
