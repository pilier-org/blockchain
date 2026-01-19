# Build stage
FROM rust:1.92-bookworm AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    git \
    clang \
    curl \
    libssl-dev \
    llvm \
    libudev-dev \
    make \
    protobuf-compiler \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Add wasm target
RUN rustup target add wasm32-unknown-unknown

# Set working directory
WORKDIR /pilier

# Copy source
COPY . .

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create user
RUN useradd -m -u 1000 -U -s /bin/sh pilier && \
    mkdir -p /data /pilier && \
    chown -R pilier:pilier /data /pilier

# Copy binary
COPY --from=builder /pilier/target/release/pilier-node /usr/local/bin/

# Set user
USER pilier

# Expose ports
EXPOSE 30333 9944 9615

# Set volume
VOLUME ["/data"]

# Entrypoint
ENTRYPOINT ["/usr/local/bin/pilier-node"]