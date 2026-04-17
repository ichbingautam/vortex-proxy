//! Proxy-Wasm ABI types and enums.

/// Represents the status code returned by Proxy-Wasm host calls.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmResult {
    /// Success
    Ok = 0,
    /// Not found
    NotFound = 1,
    /// Bad argument
    BadArgument = 2,
    /// Serialization failure
    SerializationFailure = 3,
    /// Parse failure
    ParseFailure = 4,
    /// Bad expression
    BadExpression = 5,
    /// Invalid memory access
    InvalidMemoryAccess = 6,
    /// Empty result
    Empty = 7,
    /// CAS mismatch
    CasMismatch = 8,
    /// Result mismatch
    ResultMismatch = 9,
    /// Internal failure
    InternalFailure = 10,
    /// Broken connection
    BrokenConnection = 11,
}

impl From<i32> for WasmResult {
    fn from(val: i32) -> Self {
        match val {
            0 => WasmResult::Ok,
            1 => WasmResult::NotFound,
            2 => WasmResult::BadArgument,
            3 => WasmResult::SerializationFailure,
            4 => WasmResult::ParseFailure,
            5 => WasmResult::BadExpression,
            6 => WasmResult::InvalidMemoryAccess,
            7 => WasmResult::Empty,
            _ => WasmResult::InternalFailure,
        }
    }
}

/// The action dictated by a Proxy-Wasm filter callback.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Continue execution
    Continue = 0,
    /// Pause execution
    Pause = 1,
}
