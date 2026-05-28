use std::sync::Arc;

pub mod chunker;
pub mod dashboard;
pub mod embedder;
pub mod ingestor;
pub mod llm;
pub mod reranker;
pub mod retriever;
pub mod security;
pub mod server;

#[derive(Clone)]
pub struct AppState {
    pub embedder: Arc<embedder::Embedder>,
    pub retriever: Arc<retriever::Retriever>,
    pub llm_client: Arc<llm::LlmClient>,
    pub reranker: Arc<reranker::Reranker>,
    pub chunker_config: chunker::ChunkerConfig,
    pub security: Arc<security::SecurityState>,
}
