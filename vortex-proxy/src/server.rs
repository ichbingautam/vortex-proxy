//! Server module for handling incoming connections and HTTP parsing.

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper::body::Bytes;
use http_body_util::Full;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// Starts the proxy server on the given address.
pub async fn start_server(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        // Wrap the standard TCP stream with TokioIo for hyper
        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(handle_request))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

/// Handles incoming HTTP requests and forwards them.
async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    println!("Received request: {} {}", req.method(), req.uri());

    // For now, return a static response as the foundation.
    // Zero-copy body handling will use the Incoming body directly in the forwarder logic.
    let response = Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from("Vortex Proxy: Traffic received and parsed.")))
        .unwrap();

    Ok(response)
}
