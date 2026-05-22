use anyhow::Context;
use axum::Router;
use blazerag::{chunker, embedder, llm, reranker, retriever, server, AppState};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "blazerag=info".into()),
        )
        .init();

    dotenvy::dotenv().ok();

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let qdrant_url = std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".into());
    let qdrant_collection =
        std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "documents".into());
    let llm_provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "openai".into());
    let llm_api_key = std::env::var("LLM_API_KEY").unwrap_or_else(|_| String::new());
    let llm_model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    let llm_endpoint = std::env::var("LLM_ENDPOINT")
        .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".into());
    let chunk_size: usize = std::env::var("CHUNK_SIZE")
        .unwrap_or_else(|_| "512".into())
        .parse()
        .context("CHUNK_SIZE must be a number")?;
    let chunk_overlap: usize = std::env::var("CHUNK_OVERLAP")
        .unwrap_or_else(|_| "64".into())
        .parse()
        .context("CHUNK_OVERLAP must be a number")?;

    let embedding_dim: u64 = std::env::var("EMBEDDING_DIM")
        .unwrap_or_else(|_| "384".into())
        .parse()
        .context("EMBEDDING_DIM must be a number")?;

    tracing::info!("Initializing embedder...");
    let embedder = embedder::Embedder::new()
        .await
        .context("Failed to initialize embedder")?;

    tracing::info!("Connecting to Qdrant at: {}", qdrant_url);
    let retriever = retriever::Retriever::new(&qdrant_url, &qdrant_collection, embedding_dim)
        .await
        .context("Failed to connect to Qdrant")?;

    tracing::info!(
        "Initializing LLM client: provider={}, model={}",
        llm_provider,
        llm_model
    );
    let llm_client = llm::LlmClient::new(&llm_provider, &llm_api_key, &llm_model, &llm_endpoint);

    tracing::info!("Initializing reranker...");
    let reranker = reranker::Reranker::new()
        .await
        .context("Failed to initialize reranker")?;

    let state = AppState {
        embedder: Arc::new(embedder),
        retriever: Arc::new(retriever),
        llm_client: Arc::new(llm_client),
        reranker: Arc::new(reranker),
        chunker_config: chunker::ChunkerConfig {
            chunk_size,
            chunk_overlap,
        },
    };

    let app = Router::new().merge(server::routes()).with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Blazerag server starting on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context("Failed to bind address")?;

    axum::serve(listener, app).await.context("Server error")?;

    Ok(())
}
