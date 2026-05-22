# BlazeRAG Architecture

## Overview

BlazeRAG is a high-performance Retrieval-Augmented Generation (RAG) server built in Rust using the Axum web framework and Tokio async runtime.

## Components

### HTTP Server (`src/server/`)
Axum-based server exposing three endpoints:
- `POST /ingest` â€” chunk, embed, and store documents
- `POST /query` â€” embed question, retrieve context, stream LLM answer
- `GET /health` â€” liveness probe

### Chunker (`src/chunker/`)
Splits input text into overlapping chunks using the `text-splitter` crate. Configurable via `CHUNK_SIZE` and `CHUNK_OVERLAP` env vars.

### Embedder (`src/embedder/`)
Two backends selectable via `EMBEDDING_BACKEND`:
- **HTTP** (`http.rs`): Calls HuggingFace Inference API (default)
- **ONNX** (`onnx.rs`): Runs `all-MiniLM-L6-v2` locally via `ort` (feature-gated, experimental)

### Retriever (`src/retriever/`)
Wraps the Qdrant client for vector upsert and cosine-similarity search. Collection is auto-created on startup if missing.

### LLM Client (`src/llm/`)
Thin adapter over OpenAI-compatible APIs. Supports OpenAI and Anthropic providers. Streams responses back to the caller via SSE.

## Data Flow

```
Ingest:  text â†’ chunker â†’ embedder â†’ qdrant upsert
Query:   question â†’ embedder â†’ qdrant search (top-k) â†’ context builder â†’ LLM â†’ streamed answer + sources
```

## Configuration

All settings are environment variables. See `.env.example` for the full list.
