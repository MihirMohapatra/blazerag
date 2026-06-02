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

Most RAG systems are built on Python runtimes, which means GIL contention, heavyweight processes, and slow cold starts. BlazeRAG is written in Rust, giving you:

- **Zero GIL**: true async concurrency via Tokio
- **Low memory footprint**: no interpreter overhead
- **Fast cold start**: 38 ms to first request
- **Single binary**: no virtualenvs and no Python dependency stack

If you are building a production RAG pipeline that needs to scale without throwing more hardware at it, BlazeRAG is a drop-in server to evaluate.

---

## Benchmarks

All measurements below were taken on a Windows 11 machine using the `x86_64-pc-windows-gnu` toolchain and HTTP-only embeddings.

| Metric | Measured | Notes |
|--------|----------|-------|
| Binary cold start | **38 ms** | `basic_usage` example, first run |
| Chunking throughput | **63 ops/sec** | 268 KB text, 1,111 chunks, 10k iterations |
| Chunker warmup (100x) | **349 us** | About 3.5 us/op |
| Average chunk size | **525 chars** | Config: 512 chunk size, 64 overlap |
| Compile time (release) | **1m 59s** | Full dependency tree, cold cache |

> The previous "5,000+ concurrent requests" target has not been validated by full-stack load testing yet. BlazeRAG should be benchmarked with your own Qdrant, embedding, and LLM setup before using that number for capacity planning.

Run the active benchmark with:

```bash
cargo run --release --no-default-features --example bench
```

Full-stack benchmarks with Qdrant and an LLM provider are planned for a later release.

---

## Features

- **Text ingest** via `POST /ingest`: chunks, embeds, and stores documents in Qdrant
- **Batch ingest** via `POST /ingest/batch`: PDF, HTML, and Markdown input
- **Dashboard** at `/`: ingest, batch upload, query, and streaming query UI
- **RAG query** via `POST /query`: retrieves context and calls an LLM
- **Streaming query** via `POST /query/stream`: Server-Sent Events token stream
- **Reranking**: optional HuggingFace cross-encoder reranking after vector search
- **Multi-tenancy**: tenant-isolated Qdrant collections via the `X-Tenant-ID` header
- **Modular embedders**: HuggingFace-compatible HTTP embeddings or experimental local ONNX
- **Vector search**: Qdrant cosine similarity with configurable top-k retrieval
- **LLM-compatible**: OpenAI or OpenAI-compatible chat completions endpoint
- **Docker ready**: Qdrant included in `docker-compose.yml`

---

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.77+ for native builds
- [Docker and Docker Compose](https://docs.docker.com/compose/install/) for Qdrant or containerized setup
- An LLM API key for OpenAI or an OpenAI-compatible service
- A HuggingFace token when using the default HTTP embedding endpoint

---

## Quick Start

### Docker

```bash
git clone https://github.com/MihirMohapatra/blazerag
cd blazerag
cp .env.example .env
# Edit .env and set LLM_API_KEY plus embedding credentials if needed.
docker compose up -d
```

The server starts at `http://localhost:3000`.

### Native, HTTP-Only Embeddings

This is the most portable source build and is recommended on Windows GNU because the default ONNX feature can require an MSVC-compatible linker/toolchain.

```bash
git clone https://github.com/MihirMohapatra/blazerag
cd blazerag
cp .env.example .env
# Edit .env and set LLM_API_KEY, EMBEDDING_API_KEY, and QDRANT_URL if needed.

docker compose up -d qdrant
cargo run --release --no-default-features
```

### Native, ONNX Embeddings

ONNX support is experimental and enabled by the default `onnx` feature:

```bash
docker compose up -d qdrant
cargo run --release
```

Set `EMBEDDING_BACKEND=onnx` and `ONNX_MODEL_PATH` in `.env` to use a local ONNX model.

### Cargo Install

```bash
cargo install blazerag
blazerag
```

If your platform has trouble compiling `ort`, install or run with `--no-default-features` from source.

---

## API Examples

### Ingest a Document

```bash
curl -X POST http://localhost:3000/ingest \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: acme-corp" \
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

### Ask a Question

```bash
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: acme-corp" \
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

### Health Check

```bash
curl http://localhost:3000/health
```

```json
{"status":"ok","service":"blazerag"}
```

---

## Configuration

All configuration is read from environment variables. See [.env.example](.env.example) for a complete starter file.

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address |
| `PORT` | `3000` | Server port |
| `QDRANT_URL` | `http://localhost:6333` | Qdrant endpoint |
| `QDRANT_COLLECTION` | `documents` | Base Qdrant collection name |
| `EMBEDDING_BACKEND` | `http` | `http` or `onnx` |
| `EMBEDDING_API_URL` | HuggingFace all-MiniLM-L6-v2 | HTTP embedding endpoint |
| `EMBEDDING_API_KEY` | Empty | API key for the embedding service |
| `ONNX_MODEL_PATH` | `./models/all-MiniLM-L6-v2.onnx` | Local ONNX model path |
| `EMBEDDING_DIM` | `384` | Embedding dimension used when creating Qdrant collections |
| `LLM_PROVIDER` | `openai` | Provider label used in logs; requests use the OpenAI chat completions shape |
| `LLM_API_KEY` | Empty | LLM API key |
| `LLM_MODEL` | `gpt-4o-mini` | Chat model name |
| `LLM_ENDPOINT` | OpenAI chat completions API | Chat completions endpoint |
| `CHUNK_SIZE` | `512` | Max characters per chunk |
| `CHUNK_OVERLAP` | `64` | Overlap between chunks |
| `TOP_K` | `5` | Default retrieval count when a `/query` request omits `top_k` |
| `REQUIRE_AUTH` | `false` | Require API key on ingest/query endpoints |
| `API_KEYS` | Empty | Comma-separated valid API keys |
| `RATE_LIMIT_PER_MINUTE` | `120` | Per-identity request limit |
| `RERANKER_API_URL` | HuggingFace cross-encoder | Reranker endpoint; leave empty to disable |
| `RERANKER_API_KEY` | Empty | Reranker key; falls back to `HF_API_KEY` when implemented by the provider |

---

## Testing

```bash
# Fast local unit tests
cargo test

# HTTP-only build/test path, useful on Windows GNU
cargo test --no-default-features

# CI-style all-features test path, useful on Linux or MSVC setups where ONNX builds
cargo test --all-features

# Lint and format
cargo clippy -- -D warnings
cargo fmt --check
```

Some HTTP integration tests are marked `#[ignore]` because they require Qdrant and external API credentials:

```bash
cargo test -- --ignored
```

### Test Coverage

| Module | Coverage |
|--------|----------|
| Chunker | Basic splitting, overlap, empty text |
| Embedder HTTP | Deterministic output and normalization |
| Reranker | Score parsing and result sorting |
| Ingestor | Format parsing, encoding parsing, HTML/Markdown parsing, base64 round-trip |
| Server | HTTP endpoints and SSE streaming tests, ignored by default when they need external services |

---

## API Reference

### Headers

| Header | Required | Default | Description |
|--------|----------|---------|-------------|
| `X-Tenant-ID` | No | `default` | Routes data into a tenant-specific Qdrant collection |
| `X-API-Key` | When `REQUIRE_AUTH=true` | Empty | API key auth header |
| `Authorization: Bearer <key>` | When `REQUIRE_AUTH=true` | Empty | Alternate auth header |

When `REQUIRE_AUTH=true`, missing or invalid keys return `401 Unauthorized`. Requests above `RATE_LIMIT_PER_MINUTE` return `429 Too Many Requests`.

### `POST /ingest`

Ingest text into the vector store.

Request:

```json
{
  "text": "string (required)",
  "metadata": { "source": "optional metadata object" }
}
```

Response: `200 OK`

```json
{
  "status": "ok",
  "chunks": 2,
  "ids": ["uuid-1", "uuid-2"]
}
```

### `POST /ingest/batch`

Ingest multiple PDF, HTML, or Markdown files.

Request:

```json
[
  {
    "name": "doc.pdf",
    "content": "base64-encoded content",
    "format": "pdf",
    "encoding": "base64",
    "metadata": { "source": "upload" }
  },
  {
    "name": "page.html",
    "content": "<html>...</html>",
    "format": "html",
    "encoding": "utf-8"
  }
]
```

Response: `200 OK`, or `207 Multi-Status` on partial failure

```json
{
  "status": "ok",
  "files_processed": 2,
  "total_chunks": 12,
  "results": [
    { "name": "doc.pdf", "chunks": 8 },
    { "name": "page.html", "chunks": 4 }
  ]
}
```

### `POST /query`

Ask a question using RAG.

Request:

```json
{
  "question": "string (required)",
  "top_k": 5
}
```

Response: `200 OK`

```json
{
  "answer": "string",
  "sources": [
    {
      "text": "retrieved chunk text",
      "score": 0.95,
      "id": "chunk UUID"
    }
  ]
}
```

### `POST /query/stream`

Stream a RAG answer via Server-Sent Events. The request body matches `/query`.

Response: `200 OK` with `Content-Type: text/event-stream`

```text
data: {"type":"token","content":"Blazerag"}

data: {"type":"token","content":" is"}

data: {"type":"done","sources":[{"text":"...","score":0.95,"id":"uuid-1"}]}
```

### `GET /`

Returns the web dashboard.

### `GET /health`

Response: `200 OK`

```json
{
  "status": "ok",
  "service": "blazerag"
}
```

---

## Architecture

```text
Client Request
     |
     v
[Rust HTTP Server] <- tokio + axum
     |
     v
[Query Embedder]   <- ort (ONNX) or HuggingFace API
     |
     v
[Vector Search]    <- qdrant-client, tenant collections
     |
     v
[Context Builder]  <- vector ranking plus optional reranking
     |
     v
[LLM API Call]     <- reqwest to OpenAI or a compatible endpoint
     |
     v
Response JSON or SSE stream
```

See [docs/architecture.md](docs/architecture.md) for a detailed breakdown.

### Flow Details

1. **Ingest**: Text -> chunk -> embed -> store in Qdrant
2. **Batch ingest**: PDF/HTML/Markdown -> parse -> chunk -> embed -> store
3. **Query**: Question -> embed -> vector search -> optional rerank -> build context -> generate answer
4. **Streaming**: `/query/stream` emits token events and a final `done` event with sources
5. **Multi-tenant routing**: `X-Tenant-ID` maps to `{QDRANT_COLLECTION}_{tenant}`

---

## Project Structure

```text
blazerag/
|-- .github/workflows/ci.yml
|-- src/
|   |-- main.rs          # Entry point
|   |-- lib.rs           # AppState and module exports
|   |-- server/          # Axum HTTP routes
|   |-- embedder/        # ONNX and HTTP embedding logic
|   |-- retriever/       # Qdrant vector search
|   |-- chunker/         # Text splitting
|   |-- reranker/        # Cross-encoder reranker
|   |-- ingestor/        # PDF, HTML, and Markdown parsing
|   |-- dashboard/       # Web UI dashboard
|   `-- llm/             # LLM API client
|-- docs/
|-- examples/
|-- benches/
|-- Cargo.toml
|-- Dockerfile
|-- docker-compose.yml
|-- README.md
`-- .env.example
```

---

## Development

```bash
# Watch mode; requires cargo-watch
cargo watch -x run

# Build with ONNX support; default feature, experimental
cargo build --release

# Build without ONNX; HTTP embedder only
cargo build --release --no-default-features

# Run active benchmarks
cargo run --release --no-default-features --example bench

# Generate docs
cargo doc --open
```

---

## Roadmap

- [x] Phase 0: Project setup, README, CI
- [x] Phase 1: MVP with `/ingest`, `/query`, embeddings, and vector search
- [x] Phase 2: Streaming SSE responses and server integration tests
- [x] Phase 3: Reranking
- [x] Phase 4: Batch ingestion for PDF, HTML, and Markdown
- [x] Phase 5: Multi-tenant collections
- [x] Phase 6: Auth and rate limiting
- [x] Phase 7: Web UI dashboard
- [ ] Phase 8: Managed cloud offering

---

## Contributing

1. Fork the repo.
2. Create your feature branch: `git checkout -b feat/amazing`.
3. Run checks: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`.
4. Commit: `git commit -m "feat: add amazing feature"`.
5. Push: `git push origin feat/amazing`.
6. Open a pull request.

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).

For questions or commercial licensing, open a [GitHub Discussion](https://github.com/MihirMohapatra/blazerag/discussions) or contact via GitHub.
