# Stage 1: Build the Rust binary
FROM rust:1.80-bookworm AS builder

# Install protobuf compiler (required for vortex-admin)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/vortex

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY vortex-core/Cargo.toml vortex-core/
COPY vortex-filters/Cargo.toml vortex-filters/
COPY vortex-admin/Cargo.toml vortex-admin/
COPY vortex-ebpf/Cargo.toml vortex-ebpf/
COPY vortex-proxy/Cargo.toml vortex-proxy/

# Create dummy source files to cache dependencies
RUN mkdir -p vortex-core/src vortex-core/benches vortex-filters/src vortex-admin/src vortex-ebpf/src vortex-proxy/src && \
    touch vortex-core/src/lib.rs vortex-core/benches/ewma_benchmark.rs vortex-filters/src/lib.rs vortex-admin/src/lib.rs vortex-ebpf/src/lib.rs && \
    echo 'fn main() {}' > vortex-proxy/src/main.rs

# Build dummy dependencies (this will cache all crates.io dependencies)
RUN cargo build --release

# Remove the dummy source code
RUN rm -rf vortex-core/src vortex-filters/src vortex-admin/src vortex-ebpf/src vortex-proxy/src

# Copy the actual source code
COPY . .

# Build the final release binary
# Touch the main file to force re-compilation of the proxy bin
RUN touch vortex-proxy/src/main.rs
RUN cargo build --release

# Stage 2: Minimal Runtime Environment
FROM debian:bookworm-slim

# Install runtime dependencies (e.g., OpenSSL, CA certificates)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN useradd -ms /bin/bash vortex

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/vortex/target/release/vortex-proxy /usr/local/bin/vortex-proxy

# Create directory for certs
RUN mkdir -p /etc/vortex/certs && chown -R vortex:vortex /etc/vortex

# Set user
USER vortex
WORKDIR /home/vortex

# Expose HTTPS data plane and gRPC control plane ports
EXPOSE 8443
EXPOSE 50051

# Run the proxy
ENTRYPOINT ["vortex-proxy"]
