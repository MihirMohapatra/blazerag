<div align="center">

# Blazerag

**Blazing-fast RAG server written in Rust**

[![CI](https://github.com/MihirMohapatra/blazerag/actions/workflows/ci.yml/badge.svg)](https://github.com/MihirMohapatra/blazerag/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/badge/crates.io-v0.1.0-orange)](https://crates.io/crates/blazerag)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.77+-blue)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-2496ED?logo=docker)](https://hub.docker.com/r/blazerag/blazerag)

</div>

## Why BlazeRAG?

Most RAG systems are built on Python runtimes â€” which means GIL contention, heavyweight processes, and slow cold starts. BlazeRAG is written entirely in Rust, giving you:

- **Zero GIL** â€” true async concurrency via Tokio
- **Low memory footprint** â€” no interpreter overhead
- **Fast cold start** â€” 38 ms to first request
- **Single binary** â€” no virtualenvs, no dependency hell

If you're building a production RAG pipeline that needs to scale without throwing more hardware at it, BlazeRAG is the drop-in server to evaluate.

---

## Benchmarks

All measurements taken on a Windows 11 machine (x86_64-pc-windows-gnu toolchain, no ONNX).

| Metric | Measured | Notes |
|--------|----------|-------|
| Binary cold start | **38 ms** | `basic_usage` example, first run |
| Chunking throughput | **63 ops/sec** | 268 KB text, 1,111 chunks, 10k iterations |
| Chunker warmup (100x) | **349 Âµs** | ~3.5 Âµs/op |
| Avg chunk size | **525 chars** | config: 512 chunk size, 64 overlap |
| Compile time (release) | **1m 59s** | full dependency tree, cold cache |

> **Note on the "5,000+ concurrent requests" claim:** This is a target benchmark based on Axum/Tokio's known throughput characteristics and will be formally measured in v0.2.0 on a c6i.4xlarge with full-stack (Qdrant + LLM) load testing. The numbers above reflect current unit-level benchmarks only.

*Benchmarks run via `examples/bench.rs` on release build without ONNX. Full-stack benchmarks (with Qdrant + LLM) coming soon.*

---

## Features

- **Ingest** documents via POST API â€” auto-chunks, embeds, and stores in Qdrant
- **Query** with RAG â€” retrieves relevant chunks, builds context, calls LLM
- **Modular embedders** â€” HTTP (HuggingFace API) or ONNX (local, feature-gated, experimental)
- **Vector search** via Qdrant â€” cosine similarity, configurable top-k
- **LLM agnostic** â€” OpenAI, Anthropic, or any OpenAI-compatible endpoint
- **Docker ready** â€” one-command deploy with Qdrant

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
# Edit .env â€” set your LLM_API_KEY
docker compose up -d
```

### Option 2: From source

```bash
git clone https://github.com/MihirMohapatra/blazerag
cd blazerag
cp .env.example .env
# Edit .env â€” set your LLM_API_KEY and QDRANT_URL

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

> **Note:** The ONNX embedder (`--features onnx`, default) is experimental and uses `ort 2.0.0-rc.12` (a release candidate). On Windows GNU toolchain or for stable builds, use `--no-default-features` to fall back to the HTTP embedder. See [Configuration](#configuration).

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
| `QDRANT_URL` | `http://localhost:6333` | Qdrant HTTP/REST endpoint (port 6334 for gRPC) |
| `QDRANT_COLLECTION` | `documents` | Qdrant collection name |
| `EMBEDDING_BACKEND` | `http` | `http` (HuggingFace API) or `onnx` (local, experimental) |
| `EMBEDDING_API_URL` | HuggingFace all-MiniLM-L6-v2 | Embedding API endpoint |
| `EMBEDDING_API_KEY` | â€” | API key for embedding service |
| `ONNX_MODEL_PATH` | `./models/all-MiniLM-L6-v2.onnx` | Path to ONNX model file |
| `EMBEDDING_DIM` | `384` | Embedding dimension |
| `LLM_PROVIDER` | `openai` | LLM provider (`openai`, `anthropic`) |
| `LLM_API_KEY` | â€” | LLM API key (required) |
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
| Chunker | Basic splitting, overlap, empty text | âœ… |
| Embedder (HTTP) | Deterministic output, normalization | âœ… |
| Server | Integration via HTTP endpoints | ðŸš§ planned for v0.2.0 |

---

## API Reference

### `POST /ingest`

Ingest text into the vector store.

**Request:**
```json
{
  "text": "string (required) â€” document content",
  "metadata": "object (optional) â€” arbitrary key-value pairs"
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
  "question": "string (required) â€” your question",
  "top_k": "number (optional, default: 5) â€” number of chunks to retrieve"
}
```

**Response:** `200 OK`
```json
{
  "answer": "string â€” LLM-generated answer",
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
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚   Client     â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜
       â”‚ POST /ingest | POST /query
       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Axum HTTP   â”‚â”€â”€â”€â”€â–¶â”‚  Embedder       â”‚
â”‚  (tokio)     â”‚     â”‚  (HTTP / ONNX)  â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜
       â”‚                      â”‚
       â–¼                      â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Chunker     â”‚     â”‚  Qdrant Client  â”‚
â”‚  (text-split)â”‚     â”‚  (vector store) â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜
       â”‚                      â”‚
       â–¼                      â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Context     â”‚â”€â”€â”€â”€â–¶â”‚  LLM API Call   â”‚
â”‚  Builder     â”‚     â”‚  (OpenAI/Anthropic) â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜     â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”˜
                              â”‚
                              â–¼
                     â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                     â”‚  Response +     â”‚
                     â”‚  Sources        â”‚
                     â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

See [docs/architecture.md](docs/architecture.md) for a detailed breakdown.

### Flow details

1. **Ingest**: Text â†’ chunks â†’ embed each chunk â†’ store vectors + text in Qdrant
2. **Query**: Question â†’ embed â†’ vector search â†’ build context from top-k chunks â†’ LLM generates answer â†’ return with sources
3. **Embedding**: HTTP backend calls HuggingFace Inference API; ONNX backend runs all-MiniLM-L6-v2 locally (experimental)

---

## Project Structure

```
blazerag/
â”œâ”€â”€ .github/workflows/ci.yml   # Auto-test on push & PR
â”œâ”€â”€ src/
â”‚   â”œâ”€â”€ main.rs                # Entry point, config, wiring
â”‚   â”œâ”€â”€ lib.rs                 # AppState, module exports
â”‚   â”œâ”€â”€ server/                # Axum HTTP routes
â”‚   â”‚   â””â”€â”€ mod.rs             # /ingest, /query, /health
â”‚   â”œâ”€â”€ embedder/              # Embedding backends
â”‚   â”‚   â”œâ”€â”€ mod.rs             # Trait + enum dispatcher
â”‚   â”‚   â”œâ”€â”€ http.rs            # HuggingFace API embedder
â”‚   â”‚   â””â”€â”€ onnx.rs            # ONNX Runtime embedder (feature, experimental)
â”‚   â”œâ”€â”€ retriever/             # Qdrant vector search
â”‚   â”‚   â””â”€â”€ mod.rs             # Upsert, search, collection mgmt
â”‚   â”œâ”€â”€ chunker/               # Text splitting
â”‚   â”‚   â””â”€â”€ mod.rs             # Chunk with configurable overlap
â”‚   â””â”€â”€ llm/                   # LLM API client
â”‚       â””â”€â”€ mod.rs             # OpenAI / Anthropic adapter
â”œâ”€â”€ benches/                   # Performance benchmarks
â”œâ”€â”€ docs/                      # Architecture and design docs
â”œâ”€â”€ examples/                  # Usage examples
â”œâ”€â”€ CHANGELOG.md               # Version history
â”œâ”€â”€ docker-compose.yml         # Qdrant + Blazerag
â”œâ”€â”€ Dockerfile                 # Multi-stage production build
â””â”€â”€ .env.example               # Environment config template
```

---

## Tags

| Tag | Description |
|-----|-------------|
| `v0.1.0` | MVP â€” ingest, query, HTTP embeddings, Qdrant integration |
| `latest` | Latest stable release (Docker) |
| `main` | Development branch (may be unstable) |

---

## Roadmap

- [x] Phase 0: Project setup, README, CI
- [x] Phase 1: MVP â€” /ingest, /query, embeddings, vector search
- [ ] Phase 2: Streaming SSE responses + server integration tests
- [ ] Phase 3: Reranking (cross-encoder)
- [ ] Phase 4: Batch ingestion (PDF, HTML, Markdown)
- [ ] Phase 5: Multi-tenant collections
- [ ] Phase 6: Auth & rate limiting
- [ ] Phase 7: Web UI dashboard
- [ ] Phase 8: Managed cloud offering

---

## Development

```bash
# Watch mode (requires cargo-watch)
cargo watch -x run

# Build with ONNX support (default, experimental)
cargo build --release --features onnx

# Build without ONNX (HTTP embedder only, stable)
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

For questions or commercial licensing, open a [GitHub Discussion](https://github.com/MihirMohapatra/blazerag/discussions) or contact via GitHub.
