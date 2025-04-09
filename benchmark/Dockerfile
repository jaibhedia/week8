FROM rust:latest

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    build-essential \
    cmake \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN curl -sSf https://install.surrealdb.com | sh

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -f target/release/deps/benchmark*

COPY . .

RUN cargo build --release

RUN mkdir -p ./data/rocksdb ./data/leveldb ./data/surrealdb

RUN echo '#!/bin/bash\n\
surreal start --log trace --user root --pass root --bind 0.0.0.0:8000 memory &\n\
sleep 3\n\
./target/release/benchmark\n' > /app/start.sh && chmod +x /app/start.sh

CMD ["/app/start.sh"]