//! Lock-free hot pool implementation using DashMap and SegQueue.

use std::net::SocketAddr;
use std::sync::Arc;
use dashmap::DashMap;
use crossbeam_queue::SegQueue;
use hyper::client::conn::http1::SendRequest;
use hyper::body::Incoming;

/// A lock-free two-stage hot pool for caching backend TCP connections.
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    /// Maps a backend address to a lock-free queue of idle HTTP/1.1 senders.
    idle_connections: Arc<DashMap<SocketAddr, Arc<SegQueue<SendRequest<Incoming>>>>>,
}

impl ConnectionPool {
    /// Creates a new empty connection pool.
    pub fn new() -> Self {
        Self {
            idle_connections: Arc::new(DashMap::new()),
        }
    }

    /// Tries to pop an existing, connection sender to the given backend.
    pub fn try_pop(&self, addr: &SocketAddr) -> Option<SendRequest<Incoming>> {
        if let Some(queue_ref) = self.idle_connections.get(addr) {
            let queue = queue_ref.value();
            while let Some(sender) = queue.pop() {
                // Return if the sender is not explicitly closed.
                // It still requires caller to verify `ready().await` before use.
                if !sender.is_closed() {
                    return Some(sender);
                }
            }
        }
        None
    }

    /// Pushes an active sender back into the pool for reuse.
    pub fn push(&self, addr: SocketAddr, sender: SendRequest<Incoming>) {
        if sender.is_closed() {
            return;
        }

        let queue = self.idle_connections
            .entry(addr)
            .or_insert_with(|| Arc::new(SegQueue::new()))
            .value()
            .clone();

        queue.push(sender);
    }
}
