# Build stage
FROM rust:1.83-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/llm_agent.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release --bin smart-home-llm && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the actual binary (only this rebuilds when code changes)
RUN touch src/llm_agent.rs && \
    cargo build --release --bin smart-home-llm

# Runtime stage
FROM debian:bookworm-slim

# Install CA certificates and kubectl
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    update-ca-certificates && \
    ARCH=$(dpkg --print-architecture) && \
    curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/${ARCH}/kubectl" && \
    install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl && \
    rm kubectl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/smart-home-llm /usr/local/bin/smart-home-llm

# Create non-root user
RUN useradd -m -u 1000 agent && \
    chown -R agent:agent /app

USER agent

ENTRYPOINT ["smart-home-llm"]
