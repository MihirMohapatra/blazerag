# Changelog

All notable changes to BlazeRAG are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Planned
- Streaming SSE responses
- Reranking via cross-encoder
- Batch ingestion (PDF, HTML, Markdown)
- Multi-tenant collections
- Auth & rate limiting
- Web UI dashboard

---

## [0.1.0] " 2025

### Added
- `POST /ingest` " chunk, embed, and store documents in Qdrant
- `POST /query` " RAG query with LLM-generated answer and source attribution
- `GET /health` " liveness probe
- HTTP embedder backend (HuggingFace Inference API)
- ONNX embedder backend (`all-MiniLM-L6-v2`, feature-gated, experimental)
- Qdrant vector store integration (cosine similarity, configurable top-k)
- OpenAI and Anthropic LLM provider support
- Docker Compose setup (Qdrant + BlazeRAG)
- Multi-stage Dockerfile for production builds
- GitHub Actions CI (test, clippy, fmt)
- Dual MIT / Apache-2.0 license

[Unreleased]: https://github.com/MihirMohapatra/blazerag/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/MihirMohapatra/blazerag/releases/tag/v0.1.0
