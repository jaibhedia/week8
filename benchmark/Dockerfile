FROM rust:latest

# Add necessary packages
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    build-essential \
    cmake \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install SurrealDB
RUN curl -sSf https://install.surrealdb.com | sh

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -f target/release/deps/benchmark*

# Copy source code
COPY . .

# Build release
RUN cargo build --release

# Create data directories
RUN mkdir -p /tmp/data/rocksdb /tmp/data/leveldb /tmp/data/surrealdb

# Create startup script for SurrealDB and your application
RUN echo '#!/bin/bash\n\
surreal start --log trace --user root --pass root --bind 0.0.0.0:8000 memory &\n\
sleep 3\n\
./target/release/benchmark\n' > /app/start.sh && chmod +x /app/start.sh

# Run the application with SurrealDB
CMD ["/app/start.sh"]