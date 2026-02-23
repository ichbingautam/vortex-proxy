//! Server module for handling incoming connections and HTTP parsing.

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio_rustls::TlsAcceptor;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};

// A generic boxed error type
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Starts the proxy server on the given address.
pub async fn start_server(
    addr: SocketAddr,
    tls_acceptor: Option<TlsAcceptor>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        if let Some(acceptor) = &tls_acceptor {
            let acceptor = acceptor.clone();
            tokio::task::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let io = TokioIo::new(tls_stream);
                        if let Err(err) = http1::Builder::new()
                            .serve_connection(io, service_fn(forward_request))
                            .await
                        {
                            eprintln!("Error serving connection: {:?}", err);
                        }
                    }
                    Err(e) => eprintln!("TLS Handshake failed: {}", e),
                }
            });
        } else {
            // Unencrypted fallback
            let io = TokioIo::new(stream);
            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(forward_request))
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

/// Handles incoming HTTP requests and proxies them to a static backend.
async fn forward_request(
    mut req: Request<Incoming>,
) -> Result<Response<Incoming>, BoxError> {
    println!("Proxying request: {} {}", req.method(), req.uri());

    // 1. Define static upstream backend (Mocking backend ID lookup)
    let upstream_addr = "127.0.0.1:9090";

    // 2. Establish connection to upstream
    // Note: A production Staff-level load balancer would use a connection pool (Lock-Free Hot Pool) here.
    // For Phase 1, we implement the direct zero-copy byte-pump using hyper::client::conn.
    let stream = match TcpStream::connect(upstream_addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to backend: {}", e);
            return Err(Box::new(e));
        }
    };

    let io = TokioIo::new(stream);

    // 3. Perform the HTTP/1.1 handshake with the upstream server
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(handshake) => handshake,
        Err(e) => {
            eprintln!("Failed HTTP handshake with backend: {}", e);
            return Err(Box::new(e));
        }
    };

    // 4. Spawn a task to drive the connection
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            eprintln!("Connection failed: {:?}", err);
        }
    });

    // 5. Forward the original request directly with zero-copy stream
    let uri_string = format!("http://{}{}", upstream_addr, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("/"));
    *req.uri_mut() = uri_string.parse().unwrap();
    req.headers_mut().insert(hyper::header::HOST, upstream_addr.parse().unwrap());

    let res = sender.send_request(req).await?;

    Ok(res)
}
