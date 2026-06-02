# BlazeRAG Architecture

## Overview

BlazeRAG is a high-performance Retrieval-Augmented Generation (RAG) server built in Rust using Axum and Tokio.

## Components

### HTTP Server (`src/server/`)

Axum-based server exposing:

- `GET /`: dashboard UI
- `GET /health`: liveness probe
- `POST /ingest`: chunk, embed, and store text
- `POST /ingest/batch`: parse and ingest PDF, HTML, or Markdown files
- `POST /query`: retrieve context and generate a RAG answer
- `POST /query/stream`: stream the answer via Server-Sent Events

The optional `X-Tenant-ID` header routes requests to isolated Qdrant collections.

### Chunker (`src/chunker/`)

Splits input text into overlapping chunks using `text-splitter`. Configure it with `CHUNK_SIZE` and `CHUNK_OVERLAP`.

### Embedder (`src/embedder/`)

Two backends are selectable with `EMBEDDING_BACKEND`:

- `http`: calls a HuggingFace-compatible feature extraction endpoint
- `onnx`: runs a local ONNX model via `ort` when the `onnx` feature is enabled

### Retriever (`src/retriever/`)

Wraps the Qdrant client for vector upsert and cosine-similarity search.

- Default tenant collection: `{QDRANT_COLLECTION}`
- Named tenant collection: `{QDRANT_COLLECTION}_{tenant_id}`
- Collections are created lazily on first upsert or search
- Each payload includes `tenant_id` for auditability

### Reranker (`src/reranker/`)

Optionally calls a HuggingFace cross-encoder endpoint to rescore vector search results. If reranking fails, the server falls back to the original vector scores.

### LLM Client (`src/llm/`)

Sends OpenAI-style chat completion requests and supports streaming responses. The endpoint is configurable, so OpenAI-compatible services can be used.

## Data Flow

```text
Ingest:
text -> chunker -> embedder -> Qdrant upsert

Query:
question -> embedder -> Qdrant search -> optional reranker -> context builder -> LLM -> answer + sources

Streaming query:
question -> retrieval pipeline -> LLM stream -> SSE token events -> done event with sources

Multi-tenant routing:
X-Tenant-ID -> collection name -> isolated tenant collection
```

## Multi-Tenant Design

- **Isolation level**: collection-level isolation in Qdrant
- **Header**: `X-Tenant-ID`, optional, defaults to `default`
- **Collection naming**: `{QDRANT_COLLECTION}_{tenant_id}`, except the default tenant uses `{QDRANT_COLLECTION}`
- **Lazy creation**: collections are created on first insert or search
- **Payload**: each point stores `tenant_id` for traceability

## Configuration

All settings are environment variables. See [.env.example](../.env.example) for the full list.
