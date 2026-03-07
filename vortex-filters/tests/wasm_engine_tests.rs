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

    let result = engine.execute_filter(wat.as_bytes()).expect("Failed to execute WASM module");
    assert_eq!(result, 200);
}
