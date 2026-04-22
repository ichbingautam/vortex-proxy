//! Kubernetes Gateway/CRD Watcher for Automatic Zero-Downtime Reconfiguration
//!
//! Listens to a custom CRD representing Vortex routing topologies and hot-reloads
//! the lock-free proxy engine directly.

use kube::{
    api::Api,
    runtime::{watcher, WatchStreamExt},
    Client, CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_stream::StreamExt;
use vortex_core::domain::backend::{Backend, BackendId};
use vortex_core::domain::routing::RoutingTable;

/// Represents the custom Kubernetes resource specification for Vortex Proxy routing.
#[derive(CustomResource, Debug, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[kube(
    group = "vortex.dev",
    version = "v1alpha1",
    kind = "VortexRoute",
    namespaced
)]
pub struct VortexRouteSpec {
    /// The list of backend maps specified in this route.
    pub backends: Vec<VortexBackendMap>,
}

/// A mapping definition for a proxy backend, including its ID, address, and optionally the AI models it serves.
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
pub struct VortexBackendMap {
    /// The unique identifier of the backend.
    pub id: u32,
    /// The address of the backend, e.g. "127.0.0.1:8080".
    pub address: String,
    /// The AI models served by this backend, if any.
    pub ai_models: Option<Vec<String>>,
}

/// Watches the Kubernetes APIServer for VortexRoute modifications, translating them
/// into atomic arc-swap backend rotations.
pub async fn start_k8s_watcher(
    routing_table: Arc<RoutingTable>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::try_default().await?;
    let api: Api<VortexRoute> = Api::default_namespaced(client);

    // Watch for events on VortexRoute resources
    let stream = watcher(api, watcher::Config::default()).applied_objects();
    tokio::pin!(stream);

    tracing::info!("Kubernetes Watcher initialized. Waiting for VortexRoute CRDs...");

    while let Some(event) = stream.next().await {
        match event {
            Ok(route) => {
                tracing::info!(
                    "Discovered VortexRoute update: {}",
                    route.metadata.name.as_deref().unwrap_or("unknown")
                );

                let mut new_backends = Vec::new();
                for b in route.spec.backends {
                    let addr = b
                        .address
                        .parse()
                        .unwrap_or_else(|_| "127.0.0.1:80".parse().unwrap());

                    let models = b.ai_models.unwrap_or_default();
                    new_backends.push(Arc::new(Backend::with_models(
                        BackendId(b.id),
                        addr,
                        models,
                    )));
                }

                if !new_backends.is_empty() {
                    // Execute zero-downtime atomic state swap
                    routing_table.update_backends(new_backends);
                    tracing::info!("Successfully performed atomic zero-downtime routing table reload from K8s CRD.");
                }
            }
            Err(e) => {
                tracing::error!("K8s watcher error: {}", e);
            }
        }
    }

    Ok(())
}
