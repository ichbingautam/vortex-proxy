//! Proxy-Wasm ABI types and enums.

/// Represents the status code returned by Proxy-Wasm host calls.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmResult {
    Ok = 0,
    NotFound = 1,
    BadArgument = 2,
    SerializationFailure = 3,
    ParseFailure = 4,
    BadExpression = 5,
    InvalidMemoryAccess = 6,
    Empty = 7,
    CasMismatch = 8,
    ResultMismatch = 9,
    InternalFailure = 10,
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
    Continue = 0,
    Pause = 1,
}
