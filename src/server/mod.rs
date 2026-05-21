use axum::{
    extract::State,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chunker;
use crate::retriever::Document;
use crate::AppState;

#[derive(Deserialize)]
pub struct IngestRequest {
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct IngestResponse {
    pub status: String,
    pub chunks: usize,
    pub ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub question: String,
    #[serde(default = "default_top_k")]
    pub top_k: u64,
}

fn default_top_k() -> u64 {
    5
}

#[derive(Serialize)]
pub struct QuerySource {
    pub text: String,
    pub score: f32,
    pub id: String,
}

#[derive(Serialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<QuerySource>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ingest", post(ingest_handler))
        .route("/query", post(query_handler))
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "blazerag"
    }))
}

async fn ingest_handler(
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> impl IntoResponse {
    tracing::info!("Ingesting text ({} chars)", req.text.len());

    let chunks = chunker::chunk_text(&req.text, &state.chunker_config);
    tracing::info!("Split into {} chunks", chunks.len());

    if chunks.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(
                serde_json::to_value(ErrorResponse {
                    error: "No content to ingest".into(),
                })
                .unwrap(),
            ),
        );
    }

    let metadata = req.metadata.unwrap_or(serde_json::json!({}));

    let embeddings = match state.embedder.embed(&chunks) {
        Ok(embs) => embs,
        Err(e) => {
            tracing::error!("Embedding failed: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: format!("Embedding failed: {}", e),
                    })
                    .unwrap(),
                ),
            );
        }
    };

    let ids: Vec<String> = (0..chunks.len())
        .map(|_| Uuid::new_v4().to_string())
        .collect();

    let documents: Vec<Document> = chunks
        .into_iter()
        .zip(ids.iter())
        .map(|(text, id)| Document {
            id: id.clone(),
            text,
            metadata: metadata.clone(),
        })
        .collect();

    match state.retriever.upsert(&documents, &embeddings).await {
        Ok(_) => {
            tracing::info!("Successfully stored {} chunks", documents.len());
            (
                axum::http::StatusCode::OK,
                Json(
                    serde_json::to_value(IngestResponse {
                        status: "ok".into(),
                        chunks: documents.len(),
                        ids: ids.clone(),
                    })
                    .unwrap(),
                ),
            )
        }
        Err(e) => {
            tracing::error!("Qdrant upsert failed: {}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: format!("Storage failed: {}", e),
                    })
                    .unwrap(),
                ),
            )
        }
    }
}

async fn query_handler(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> impl IntoResponse {
    tracing::info!("Query: {}", req.question);

    let query_embeddings = match state.embedder.embed(std::slice::from_ref(&req.question)) {
        Ok(embs) => embs,
        Err(e) => {
            tracing::error!("Query embedding failed: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: format!("Query embedding failed: {}", e),
                    })
                    .unwrap(),
                ),
            )
                .into_response();
        }
    };

    let query_vector = &query_embeddings[0];

    let search_results = match state.retriever.search(query_vector, req.top_k).await {
        Ok(results) => results,
        Err(e) => {
            tracing::error!("Vector search failed: {}", e);
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    serde_json::to_value(ErrorResponse {
                        error: format!("Search failed: {}", e),
                    })
                    .unwrap(),
                ),
            )
                .into_response();
        }
    };

    let sources: Vec<QuerySource> = search_results
        .iter()
        .map(|sp| {
            let text = sp
                .payload
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            QuerySource {
                text,
                score: sp.score,
                id: sp
                    .id
                    .clone()
                    .map(|id| format!("{:?}", id))
                    .unwrap_or_default(),
            }
        })
        .collect();

    if sources.is_empty() {
        return (
            axum::http::StatusCode::OK,
            Json(
                serde_json::to_value(QueryResponse {
                    answer: "I couldn't find any relevant documents to answer your question."
                        .into(),
                    sources: vec![],
                })
                .unwrap(),
            ),
        )
            .into_response();
    }

    let context: String = sources
        .iter()
        .enumerate()
        .map(|(i, s)| format!("[Source {}]: {}\n", i + 1, s.text))
        .collect();

    let system_prompt = "You are a precise RAG assistant. Answer the user's question based \
                         solely on the provided context. If the context doesn't contain \
                         enough information, say so. Cite sources when possible.";

    let user_prompt = format!(
        "Context:\n{}\n\nQuestion: {}\n\nAnswer:",
        context, req.question
    );

    match state.llm_client.generate(system_prompt, &user_prompt).await {
        Ok(answer) => (
            axum::http::StatusCode::OK,
            Json(serde_json::to_value(QueryResponse { answer, sources }).unwrap()),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("LLM generation failed: {}", e);
            // Return context-only response if LLM fails
            let fallback = format!(
                "Based on the retrieved documents:\n\n{}",
                sources
                    .iter()
                    .map(|s| format!("- {}", s.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            (
                axum::http::StatusCode::OK,
                Json(
                    serde_json::to_value(QueryResponse {
                        answer: fallback,
                        sources,
                    })
                    .unwrap(),
                ),
            )
                .into_response()
        }
    }
}
