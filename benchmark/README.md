# Database Benchmarking Application

This project provides a comprehensive benchmarking suite for comparing the performance of various database systems:
- LevelDB (key-value store)
- RocksDB (key-value store)
- SurrealDB (document/graph database)
- PostgreSQL (relational database)
- MongoDB (document database)

## Overview

The application fetches rune pool data from a remote API and stores it in multiple database systems. It then provides endpoints to benchmark CRUD operations across these databases, allowing for performance comparison.

## Features

- **Multiple Database Support**: Test and compare five different database technologies
- **Benchmark Measurements**: Measure and report operation times for each database
- **API Endpoints**: HTTP API for running benchmarks and viewing results
- **Custom Data Size**: Configurable data volume for scalability testing

## Getting Started

### Prerequisites

- Rust toolchain (recommended: latest stable version)
- Docker (for containerized deployment)
- PostgreSQL server (for PostgreSQL benchmarks)
- MongoDB server (for MongoDB benchmarks)

### Configuration

The application uses environment variables for configuration. Create a `.env` file with:

```
API_URL=https://midgard.ninerealms.com/v2/history/runepool
INTERVAL=hour
ROCKSDB_PATH=/path/to/rocksdb
LEVELDB_PATH=/path/to/leveldb
SURREALDB_URL=127.0.0.1:8000
PSQL_CONN=postgres://user:password@localhost:5432/runepool
MONGODB_URI=mongodb://localhost:27017/runepool
DB_NAME=runepool
HOST=0.0.0.0
PORT=3000
```

### Running Locally

```bash
cargo run --release
```

### Docker Deployment

```bash
docker build -t db-benchmark .
docker run -p 3000:3000 db-benchmark
```

### Render Deployment

1. Push this repository to GitHub
2. In Render, create a new Web Service
3. Connect to your GitHub repository
4. Set the build command: `cargo build --release`
5. Set the start command: `./start.sh`
6. Add environment variables as needed

## API Endpoints

| Endpoint | Method | Description | Request Body | Query Params |
|----------|--------|-------------|--------------|--------------|
| `/update` | POST | Update rune pool data | ApiRunePoolResponse | - |
| `/get` | GET | Get rune pool data | - | `db=leveldb\|rocksdb\|surrealdb\|psql\|mongodb` |
| `/fetch-and-update` | POST | Fetch data from source API and update all DBs | `{"count": number}` | - |
| `/clear` | DELETE | Clear all databases | - | - |

## Testing After Deployment

### Basic Tests

```bash
# Health check
curl https://your-app-name.onrender.com/health

# Fetch and update with custom count
curl -X POST https://your-app-name.onrender.com/fetch-and-update \
  -H "Content-Type: application/json" \
  -d '{"count": 1000}'

# Get data from specific database
curl https://your-app-name.onrender.com/get?db=rocksdb

# Clear all databases
curl -X DELETE https://your-app-name.onrender.com/clear
```

### Performance Testing

1. Clear databases: `curl -X DELETE https://your-app-name.onrender.com/clear`
2. Run tests with different data sizes:
   ```bash
   curl -X POST https://your-app-name.onrender.com/fetch-and-update \
     -H "Content-Type: application/json" \
     -d '{"count": 100}'
   ```
3. Compare the timing results returned in the response

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.