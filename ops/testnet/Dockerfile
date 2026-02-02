# Pilier Node - Production Dockerfile
FROM docker.io/paritytech/ci-unified:bullseye-latest AS builder

WORKDIR /pilier

# Build cache: dependencies first
COPY Cargo.toml Cargo.lock ./
COPY runtime/Cargo.toml runtime/Cargo.toml
COPY node/Cargo.toml node/Cargo.toml

# Create dummy source to cache dependencies
RUN mkdir -p runtime/src node/src && \
    echo "fn main() {}" > node/src/main.rs && \
    echo "pub fn dummy() {}" > runtime/src/lib.rs && \
    cargo build --release && \
    rm -rf target/release/.fingerprint/pilier-* target/release/deps/pilier*

# Copy real source code
COPY runtime runtime
COPY node node

# Build actual binary
RUN cargo build --release

# Runtime stage (smaller final image)
FROM docker.io/parity/base-bin:latest

WORKDIR /pilier

# Copy binary from builder
COPY --from=builder /pilier/target/release/pilier-node /usr/local/bin/

# Verify binary works
RUN pilier-node --version

# Expose ports
EXPOSE 30333 9944 9615

ENTRYPOINT ["pilier-node"]
