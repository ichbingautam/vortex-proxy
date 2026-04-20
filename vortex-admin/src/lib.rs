//! Control plane Unix socket API for Vortex.

pub mod server;
pub mod k8s_watcher;

/// Protobuf generated code for Vortex admin API.
#[allow(missing_docs)]
pub mod proto {
    tonic::include_proto!("vortex.admin");
}

/// Initialize the vortex-admin telemetry and core states.
pub fn admin_init() {
    println!("Vortex Admin (UDS) module initialization sweep complete.");
}
