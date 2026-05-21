# Blazerag Agent Guidance

## Essential Commands
- **Test**: `cargo test --all-features` (CI) or `cargo test` (local)
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt --check`
- **Build Release**: `cargo build --release --locked`
- **Run Example**: `cargo run --release --no-default-features --example basic_usage`
- **Run Benchmarks**: `cargo run --release --no-default-features --example bench`

## Critical Constraints
- **Windows GNU Toolchain**: Default `x86_64-pc-windows-gnu` lacks MSVC linker
  - ONNX embedder (`ort`) fails to compile - use `--no-default-features` for HTTP-only mode
  - CI runs on Ubuntu (Linux) where ONNX works
- **Feature Flags**: 
  - Default: `["onnx"]` enables ONNX embedder
  - HTTP-only: `--no-default-features` disables ONNX, uses HuggingFace API
- **Qdrant API**: v1.18 uses builder pattern (no `.await` on client)
  - `Qdrant::from_url(url).build()` 
  - `UpsertPointsBuilder`, `SearchPointsBuilder`

## Project Structure
- **Entrypoint**: `src/main.rs` → loads env, builds `AppState`, starts Axum server
- **AppState**: Defined in `src/lib.rs`, shared via `with_state()`
- **Modules**:
  - `server`: Axum routes (`/ingest`, `/query`, `/health`)
  - `embedder`: HTTP (HF API) or ONNX (local) via trait/enum
  - `retriever`: Qdrant client (v1.18 builder API)
  - `chunker`: Text splitting with overlap (text-splitter v0.14)
  - `llm`: OpenAI/Anthropic compatible client
- **Benchmarks**: 
  - Active: `examples/bench.rs` (run via `cargo run --example bench`)
  - Reference: `benches/benchmark.rs` (not used by `cargo bench`)

## Testing Notes
- Unit tests exist but are `#[ignore]`d by default
- To run: `cargo test -- --ignored` or remove `#[ignore]` from specific tests
- No integration tests (requires Qdrant + LLM API keys)
- Test sequence: `cargo test --all-features && cargo clippy -- -D warnings && cargo fmt --check`

## Common Gotchas
- **Env Loading**: Uses `dotenvy::dotenv().ok()` in `main()`
- **Chunking**: `text-splitter` v0.14 API - `TextSplitter::new(size).chunks(text)`
- **Embedder Switch**: `Embedder::new()` picks HTTP if `ort` unavailable/features disabled
- **Mutex Requirement**: `ort` 2.0.0-rc.12 `Session::run` needs `&mut self` → wrapped in `Mutex<Session>`
- **CLI Help**: Examples show usage (`basic_usage` prints instructions when run)

## Verification
- Check build: `cargo check --all-features`
- Verify deps: `cargo tree -i ort` (see optional dependency)
- Confirm features: `cargo +stable rustc -- --print cfg` (look for `feature=\"onnx\"`)