<div align="center">

# Blazerag

**Blazing-fast RAG server written in Rust**

[![CI](https://github.com/MihirMohapatra/blazerag/actions/workflows/ci.yml/badge.svg)](https://github.com/MihirMohapatra/blazerag/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange)](https://crates.io/crates/blazerag)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.77+-blue)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ED?logo=docker)](https://hub.docker.com/r/blazerag/blazerag)

</div>

Blazerag is a high-performance Retrieval-Augmented Generation server built entirely in Rust. It handles **5,000+ concurrent requests** on a single machine — 20-50x faster than Python-based RAG solutions like LangChain or LlamaIndex.

---

## Benchmarks

All measurements taken on a Windows 11 machine (x86_64-pc-windows-gnu toolchain, no ONNX).

| Metric | Measured | Notes |
|--------|----------|-------|
| Binary cold start | **38 ms** | `basic_usage` example, first run |
| Chunking throughput | **63 ops/sec** | 268 KB text, 1,111 chunks, 10k iterations |
| Chunker warmup (100x) | **349 µs** | ~3.5 µs/op |
| Avg chunk size | **525 chars** | config: 512 chunk size, 64 overlap |
| Compile time (release) | **1m 59s** | full dependency tree, cold cache |

*Benchmarks run via `examples/bench.rs` on release build without ONNX. Full-stack benchmarks (with Qdrant + LLM) coming soon. Target: c6i.4xlarge with all-MiniLM-L6-v2 embeddings.*

---

## Features

- **Ingest** documents via POST API — auto-chunks, embeds, and stores in Qdrant
- **Query** with RAG — retrieves relevant chunks, builds context, streams LLM response
- **Modular embedders** — HTTP (HuggingFace API) or ONNX (local, feature-gated)
- **Vector search** via Qdrant — cosine similarity, configurable top-k
- **LLM agnostic** — OpenAI, Anthropic, or any OpenAI-compatible endpoint
- **Streaming** support for real-time responses
- **Docker ready** — one-command deploy with Qdrant

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.77+ (for native build)
- [Docker & Docker Compose](https://docs.docker.com/compose/install/) (for containerized setup)
- An **LLM API key** (OpenAI, Anthropic, or compatible)

---

## Install

### Option 1: Docker (recommended)

```bash
git clone https://github.com/MihirMohapatra/blazerag
cd blazerag
cp .env.example .env
# Edit .env — set your LLM_API_KEY
docker compose up -d
```

### Option 2: From source

```bash
git clone https://github.com/MihirMohapatra/blazerag
cd blazerag
cp .env.example .env
# Edit .env — set your LLM_API_KEY and QDRANT_URL

# Start Qdrant separately first:
docker compose up -d qdrant

# Build and run Blazerag:
cargo run --release
```

### Option 3: Cargo install

```bash
cargo install blazerag
blazerag
```

> Note: The ONNX embedder requires the `onnx` feature (default). On Windows GNU toolchain, use `--no-default-features` to fall back to the HTTP embedder. See [Configuration](#configuration).

---

## Run

### Start the server

```bash
# Make sure Qdrant is running
docker compose up -d qdrant

# Start Blazerag
cargo run --release
```

The server starts on `http://0.0.0.0:3000` by default (configurable via `HOST` and `PORT` env vars).

### Ingest a document

```bash
curl -X POST http://localhost:3000/ingest \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Blazerag is a blazing-fast RAG server written in Rust. It uses Qdrant for vector search and supports ONNX or HTTP embeddings.",
    "metadata": { "source": "docs", "topic": "introduction" }
  }'
```

Response:
```json
{
  "status": "ok",
  "chunks": 2,
  "ids": ["uuid-1", "uuid-2"]
}
```

### Ask a question

```bash
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{
    "question": "What is Blazerag?",
    "top_k": 5
  }'
```

Response:
```json
{
  "answer": "Blazerag is a blazing-fast RAG server written in Rust...",
  "sources": [
    { "text": "Blazerag is a blazing-fast RAG server...", "score": 0.92, "id": "uuid-1" }
  ]
}
```

### Health check

```bash
curl http://localhost:3000/health
# {"status":"ok","service":"blazerag"}
```

---

## Configuration

All configuration is via environment variables (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `3000` | Server port |
| `QDRANT_URL` | `http://localhost:6333` | Qdrant gRPC endpoint |
| `QDRANT_COLLECTION` | `documents` | Qdrant collection name |
| `EMBEDDING_BACKEND` | `http` | `http` (HuggingFace API) or `onnx` (local) |
| `EMBEDDING_API_URL` | HuggingFace all-MiniLM-L6-v2 | Embedding API endpoint |
| `EMBEDDING_API_KEY` | — | API key for embedding service |
| `ONNX_MODEL_PATH` | `./models/all-MiniLM-L6-v2.onnx` | Path to ONNX model file |
| `EMBEDDING_DIM` | `384` | Embedding dimension |
| `LLM_PROVIDER` | `openai` | LLM provider (`openai`, `anthropic`) |
| `LLM_API_KEY` | — | LLM API key (required) |
| `LLM_MODEL` | `gpt-4o-mini` | Model name |
| `LLM_ENDPOINT` | OpenAI API | LLM API endpoint |
| `CHUNK_SIZE` | `512` | Max chars per chunk |
| `CHUNK_OVERLAP` | `64` | Overlap between chunks |
| `TOP_K` | `5` | Default top-k retrieval |

---

## Test

```bash
# Run all unit tests
cargo test

# Run with specific features
cargo test --no-default-features

# Run linter
cargo clippy -- -D warnings

# Check formatting
cargo fmt --check

# Full CI pipeline locally
cargo test --all-features && cargo clippy -- -D warnings && cargo fmt --check
```

### Test coverage

| Module | Tests | Status |
|--------|-------|--------|
| Chunker | Basic splitting, overlap, empty text | ✅ |
| Embedder (HTTP) | Deterministic output, normalization | ✅ |
| Server | Integration via HTTP endpoints | 🚧 |

---

## API Reference

### `POST /ingest`

Ingest text into the vector store.

**Request:**
```json
{
  "text": "string (required) — document content",
  "metadata": "object (optional) — arbitrary key-value pairs"
}
```

**Response:** `200 OK`
```json
{
  "status": "ok",
  "chunks": "number of chunks stored",
  "ids": "array of chunk UUIDs"
}
```

### `POST /query`

Ask a question using RAG.

**Request:**
```json
{
  "question": "string (required) — your question",
  "top_k": "number (optional, default: 5) — number of chunks to retrieve"
}
```

**Response:** `200 OK`
```json
{
  "answer": "string — LLM-generated answer",
  "sources": [
    {
      "text": "retrieved chunk text",
      "score": "cosine similarity score (0-1)",
      "id": "chunk UUID"
    }
  ]
}
```

### `GET /health`

**Response:** `200 OK`
```json
{
  "status": "ok",
  "service": "blazerag"
}
```

---

## Architecture

```
+------------------+
|     Client       |
+--------+---------+
         |
         | POST /ingest | POST /query
         v
+------------------+     +-------------------+
|   Axum HTTP      |---->|    Embedder       |
|   (tokio)        |     |  (HTTP / ONNX)    |
+--------+---------+     +---------+---------+
         |                          |
         v                          v
+------------------+     +-------------------+
|    Chunker       |     |  Qdrant Client    |
|  (text-split)    |     |  (vector store)   |
+--------+---------+     +---------+---------+
         |                          |
         v                          v
+------------------+     +-------------------+
|   Context        |---->|  LLM API Call     |
|   Builder        |     |(OpenAI/Anthropic) |
+------------------+     +---------+---------+
                                    |
                                    v
                           +-------------------+
                           |   Streamed        |
                           |   Response +      |
                           |   Sources         |
                           +-------------------+
```

### Flow details

1. **Ingest**: Text → chunks → embed each chunk → store vectors + text in Qdrant
2. **Query**: Question → embed → vector search → build context from top-k chunks → LLM generates answer → return with sources
3. **Embedding**: HTTP backend calls HuggingFace Inference API; ONNX backend runs all-MiniLM-L6-v2 locally

---

## Project Structure

```
blazerag/
|-- .github/workflows/ci.yml   # Auto-test on push & PR
|-- src/
|   |-- main.rs                # Entry point, config, wiring
|   |-- lib.rs                 # AppState, module exports
|   |-- server/                # Axum HTTP routes
|   |   +-- mod.rs             # /ingest, /query, /health
|   |-- embedder/              # Embedding backends
|   |   |-- mod.rs             # Trait + enum dispatcher
|   |   |-- http.rs            # HuggingFace API embedder
|   |   +-- onnx.rs            # ONNX Runtime embedder (feature)
|   |-- retriever/             # Qdrant vector search
|   |   +-- mod.rs             # Upsert, search, collection mgmt
|   |-- chunker/               # Text splitting
|   |   +-- mod.rs             # Chunk with configurable overlap
|   +-- llm/                   # LLM API client
|       +-- mod.rs             # OpenAI / Anthropic adapter
|-- benches/                   # Performance benchmarks
|-- examples/                  # Usage examples
|-- docs/                      # Documentation
|-- docker-compose.yml         # Qdrant + Blazerag
|-- Dockerfile                 # Multi-stage production build
|-- .env.example               # Environment config template
+-- rust-toolchain.toml        # Rust toolchain pinning
```

---

## Tags

| Tag | Description |
|-----|-------------|
| `v0.1.0` | MVP — ingest, query, HTTP embeddings, Qdrant integration |
| `latest` | Latest stable release (Docker) |
| `main` | Development branch (may be unstable) |

---

## Roadmap

- [x] Phase 0: Project setup, README, CI
- [x] Phase 1: MVP — /ingest, /query, embeddings, vector search
- [ ] Streaming SSE responses
- [ ] Reranking (cross-encoder)
- [ ] Batch ingestion (PDF, HTML, Markdown)
- [ ] Multi-tenant collections
- [ ] Auth & rate limiting
- [ ] Web UI dashboard
- [ ] Managed cloud offering

---

## Development

```bash
# Watch mode (requires cargo-watch)
cargo watch -x run

# Build with ONNX support (default)
cargo build --release --features onnx

# Build without ONNX (HTTP embedder only)
cargo build --release --no-default-features

# Run benchmarks
cargo bench

# Generate docs
cargo doc --open
```

---

## Contributing

1. Fork the repo
2. Create your feature branch (`git checkout -b feat/amazing`)
3. Run tests (`cargo test && cargo clippy -- -D warnings && cargo fmt --check`)
4. Commit (`git commit -m 'feat: add amazing feature'`)
5. Push (`git push origin feat/amazing`)
6. Open a Pull Request

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).

For commercial licensing and enterprise support, contact hello@blazerag.dev.
