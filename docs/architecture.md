# BlazeRAG Architecture

## Overview

BlazeRAG is a high-performance Retrieval-Augmented Generation (RAG) server built in Rust using the Axum web framework and Tokio async runtime.

## Components

### HTTP Server (`src/server/`)
Axum-based server exposing endpoints:
- `POST /ingest` " chunk, embed, and store documents (tenant-isolated)
- `POST /query` " embed question, retrieve context, stream LLM answer (tenant-isolated)
- `POST /query/stream` " SSE streaming query
- `GET /health` " liveness probe
- `X-Tenant-ID` header routes requests to isolated Qdrant collections

### Chunker (`src/chunker/`)
Splits input text into overlapping chunks using the `text-splitter` crate. Configurable via `CHUNK_SIZE` and `CHUNK_OVERLAP` env vars.

### Embedder (`src/embedder/`)
Two backends selectable via `EMBEDDING_BACKEND`:
- **HTTP** (`http.rs`): Calls HuggingFace Inference API (default)
- **ONNX** (`onnx.rs`): Runs `all-MiniLM-L6-v2` locally via `ort` (feature-gated, experimental)

### Retriever (`src/retriever/`)
Wraps the Qdrant client for vector upsert and cosine-similarity search. Supports multi-tenant isolation:

- Collection name = `{prefix}_{tenant_id}`, or just `{prefix}` for the "default" tenant
- Collections are created lazily on first upsert or search per tenant
- Each document payload includes a `tenant_id` field for auditability

### Reranker (`src/reranker/`)
Optional cross-encoder reranker (HuggingFace) that re-scores vector search results for improved relevance. Gracefully falls back to vector scores on error.

### LLM Client (`src/llm/`)
Thin adapter over OpenAI-compatible APIs. Supports OpenAI and Anthropic providers. Streams responses back to the caller via SSE.

## Data Flow

```
Ingest:  text ' chunker ' embedder ' qdrant upsert (tenant collection)
Query:   question ' embedder ' qdrant search (tenant collection) ' reranker ' context builder ' LLM ' streamed answer + sources
Multi-tenant: X-Tenant-ID header ' collection routing ' isolated per-tenant Qdrant shards
```

## Multi-Tenant Design

- **Isolation level**: Collection-level (each tenant gets a separate Qdrant collection)
- **Header**: `X-Tenant-ID` optional HTTP header; defaults to `"default"`
- **Collection naming**: `{QDRANT_COLLECTION}_{tenant_id}` (e.g., `documents_acme-corp`)
- **Lazy creation**: Collections auto-created on first insert or search for that tenant
- **Payload**: Each point stored with a `tenant_id` field for additional filtering if needed
- **No shared state**: Tenants cannot access each other's data since each collection is independent

## Configuration

All settings are environment variables. See `.env.example` for the full list.