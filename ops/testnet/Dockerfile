# Pilier Node - Production Dockerfile
FROM docker.io/paritytech/ci-unified:latest AS builder

WORKDIR /pilier

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY runtime/Cargo.toml runtime/Cargo.toml
COPY node/Cargo.toml node/Cargo.toml

# Copy pallets (просто COPY, без bash)
COPY pallets ./pallets

# Create dummy sources
RUN mkdir -p runtime/src node/src && \
    echo "fn main() {}" > node/src/main.rs && \
    echo "pub fn dummy() {}" > runtime/src/lib.rs

# Build dependencies
RUN cargo build --release && \
    rm -rf target/release/.fingerprint/pilier-* && \
    rm -rf target/release/deps/pilier*

# Copy real source
COPY runtime runtime
COPY node node
COPY pallets pallets

# Build
RUN cargo build --release

# Runtime stage
FROM docker.io/parity/base-bin:latest

WORKDIR /pilier

COPY --from=builder /pilier/target/release/pilier-node /usr/local/bin/

RUN pilier-node --version

EXPOSE 30333 9944 9615

ENTRYPOINT ["pilier-node"]