FROM rust:1.74-slim

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -f target/release/deps/benchmark*

COPY . .

RUN cargo build --release

RUN mkdir -p /tmp/data/rocksdb /tmp/data/leveldb

CMD ["./target/release/benchmark"]