use std::pin::Pin;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::sse::{Event, Sse},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use futures::stream::{self, Stream};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::str::FromStr;

use crate::chunker;
use crate::ingestor::{self, Encoding, FileFormat};
use crate::reranker::Reranker;
use crate::retriever::Document;
use crate::AppState;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct QueryRequest {
    pub question: String,
    #[serde(default = "default_top_k")]
    pub top_k: u64,
}

fn default_top_k() -> u64 {
    5
}

fn extract_tenant_id(headers: &HeaderMap) -> String {
    headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuerySource {
    pub text: String,
    pub score: f32,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<QuerySource>,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

type SseStream = Pin<Box<dyn Stream<Item = Result<Event, anyhow::Error>> + Send>>;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/ingest", post(ingest_handler))
        .route("/ingest/batch", post(batch_ingest_handler))
        .route("/query", post(query_handler))
        .route("/query/stream", post(query_stream_handler))
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "blazerag"
    }))
}

async fn ingest_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<IngestRequest>,
) -> impl IntoResponse {
    let tenant_id = extract_tenant_id(&headers);
    tracing::info!(
        "Ingesting text ({} chars) for tenant={}",
        req.text.len(),
        tenant_id
    );

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

    match state
        .retriever
        .upsert(&tenant_id, &documents, &embeddings)
        .await
    {
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

#[derive(Serialize, Deserialize)]
pub struct BatchFileEntry {
    pub name: String,
    pub content: String,
    pub format: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn default_encoding() -> String {
    "utf-8".into()
}

#[derive(Serialize)]
pub struct FileResult {
    pub name: String,
    pub chunks: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct BatchIngestResponse {
    pub status: String,
    pub files_processed: usize,
    pub total_chunks: usize,
    pub results: Vec<FileResult>,
}

async fn batch_ingest_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<Vec<BatchFileEntry>>,
) -> impl IntoResponse {
    let tenant_id = extract_tenant_id(&headers);
    let total = req.len();
    tracing::info!("Batch ingest: {} files for tenant={}", total, tenant_id);

    let mut results = Vec::with_capacity(total);
    let mut overall_chunks = 0usize;
    let mut has_error = false;

    for file in req {
        let format = match FileFormat::from_str(&file.format) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("Skipping {}: {}", file.name, e);
                results.push(FileResult {
                    name: file.name,
                    chunks: 0,
                    error: Some(e.to_string()),
                });
                has_error = true;
                continue;
            }
        };

        let encoding = match Encoding::from_str(&file.encoding) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Skipping {}: {}", file.name, e);
                results.push(FileResult {
                    name: file.name,
                    chunks: 0,
                    error: Some(e.to_string()),
                });
                has_error = true;
                continue;
            }
        };

        let text = match ingestor::parse_text(&file.content, format, encoding) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Skipping {}: {}", file.name, e);
                results.push(FileResult {
                    name: file.name,
                    chunks: 0,
                    error: Some(e.to_string()),
                });
                has_error = true;
                continue;
            }
        };

        let chunks = chunker::chunk_text(&text, &state.chunker_config);
        if chunks.is_empty() {
            results.push(FileResult {
                name: file.name,
                chunks: 0,
                error: Some("No content after parsing".into()),
            });
            has_error = true;
            continue;
        }

        let embeddings = match state.embedder.embed(&chunks) {
            Ok(embs) => embs,
            Err(e) => {
                tracing::error!("Embedding failed for {}: {}", file.name, e);
                results.push(FileResult {
                    name: file.name,
                    chunks: 0,
                    error: Some(format!("Embedding failed: {}", e)),
                });
                has_error = true;
                continue;
            }
        };

        let ids: Vec<String> = (0..chunks.len())
            .map(|_| Uuid::new_v4().to_string())
            .collect();

        let metadata = file
            .metadata
            .unwrap_or_else(|| serde_json::json!({"source": file.name, "format": file.format}));

        let documents: Vec<Document> = chunks
            .into_iter()
            .zip(ids.iter())
            .map(|(text, id)| Document {
                id: id.clone(),
                text,
                metadata: metadata.clone(),
            })
            .collect();

        match state
            .retriever
            .upsert(&tenant_id, &documents, &embeddings)
            .await
        {
            Ok(_) => {
                tracing::info!("Stored {} chunks from {}", documents.len(), file.name);
                overall_chunks += documents.len();
                results.push(FileResult {
                    name: file.name,
                    chunks: documents.len(),
                    error: None,
                });
            }
            Err(e) => {
                tracing::error!("Qdrant upsert failed for {}: {}", file.name, e);
                results.push(FileResult {
                    name: file.name,
                    chunks: 0,
                    error: Some(format!("Storage failed: {}", e)),
                });
                has_error = true;
            }
        }
    }

    let status_code = if has_error {
        axum::http::StatusCode::MULTI_STATUS
    } else {
        axum::http::StatusCode::OK
    };

    (
        status_code,
        Json(
            serde_json::to_value(BatchIngestResponse {
                status: if has_error {
                    "partial".to_string()
                } else {
                    "ok".to_string()
                },
                files_processed: total,
                total_chunks: overall_chunks,
                results,
            })
            .unwrap(),
        ),
    )
}

async fn query_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> impl IntoResponse {
    let tenant_id = extract_tenant_id(&headers);
    tracing::info!("Query: {} (tenant={})", req.question, tenant_id);

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

    let search_results = match state
        .retriever
        .search(&tenant_id, query_vector, req.top_k)
        .await
    {
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

    let sources = rerank_sources(&state.reranker, &req.question, sources).await;

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

async fn query_stream_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Sse<SseStream>, (StatusCode, Json<ErrorResponse>)> {
    let tenant_id = extract_tenant_id(&headers);
    tracing::info!("Streaming query: {} (tenant={})", req.question, tenant_id);

    let query_embeddings = state
        .embedder
        .embed(std::slice::from_ref(&req.question))
        .map_err(|e| {
            tracing::error!("Query embedding failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Query embedding failed: {}", e),
                }),
            )
        })?;

    let search_results = state
        .retriever
        .search(&tenant_id, &query_embeddings[0], req.top_k)
        .await
        .map_err(|e| {
            tracing::error!("Vector search failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Search failed: {}", e),
                }),
            )
        })?;

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

    let sources = rerank_sources(&state.reranker, &req.question, sources).await;

    if sources.is_empty() {
        let done = serde_json::json!({"type": "done", "sources": []});
        let stream: SseStream = Box::pin(stream::once(async move {
            Ok(Event::default().data(done.to_string()))
        }));
        return Ok(Sse::new(stream));
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

    let llm_stream = state
        .llm_client
        .generate_stream(system_prompt, &user_prompt)
        .await
        .map_err(|e| {
            tracing::error!("LLM stream failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("LLM generation failed: {}", e),
                }),
            )
        })?;

    let token_events = llm_stream.map(|result| {
        result.map(|token| {
            let payload = serde_json::json!({"type": "token", "content": token});
            Event::default().data(payload.to_string())
        })
    });

    let sources_clone = sources.clone();
    let done_event = stream::once(async move {
        let payload = serde_json::json!({"type": "done", "sources": sources_clone});
        Ok(Event::default().data(payload.to_string()))
    });

    let chained: SseStream = Box::pin(token_events.chain(done_event));
    Ok(Sse::new(chained))
}

async fn rerank_sources(
    reranker: &Reranker,
    query: &str,
    sources: Vec<QuerySource>,
) -> Vec<QuerySource> {
    let texts: Vec<(String, String)> = sources
        .iter()
        .map(|s| (s.id.clone(), s.text.clone()))
        .collect();
    match reranker.rerank(query, &texts).await {
        Ok(reranked) => reranked
            .into_iter()
            .map(|(id, text, score)| QuerySource { id, text, score })
            .collect(),
        Err(e) => {
            tracing::warn!("Reranking failed (falling back to vector scores): {}", e);
            sources
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::ChunkerConfig;
    use crate::embedder::Embedder;
    use crate::llm::LlmClient;
    use crate::reranker::Reranker;
    use crate::retriever::Retriever;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state() -> AppState {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let embedder = rt.block_on(async {
            std::env::set_var("EMBEDDING_API_URL", "http://localhost:9999/fake");
            Embedder::new().await.unwrap()
        });
        let retriever = rt.block_on(async {
            Retriever::new("http://localhost:6333", "test_collection", 384).await
        });
        let llm_client = LlmClient::new("openai", "", "test-model", "http://localhost:9999/v1");
        let reranker = rt.block_on(async { Reranker::new().await.unwrap() });

        let chunker_config = ChunkerConfig {
            chunk_size: 512,
            chunk_overlap: 64,
        };

        AppState {
            embedder: Arc::new(embedder),
            retriever: Arc::new(retriever.unwrap_or_else(|_| {
                panic!("Qdrant must be running on localhost:6333 for integration tests")
            })),
            llm_client: Arc::new(llm_client),
            reranker: Arc::new(reranker),
            chunker_config,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_health_check() {
        let app = routes().with_state(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    #[ignore]
    async fn test_ingest_empty_text() {
        let app = routes().with_state(test_state());
        let req = IngestRequest {
            text: String::new(),
            metadata: None,
        };
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/ingest")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_no_sources() {
        let app = routes().with_state(test_state());
        let req = QueryRequest {
            question: "nonexistent topic xyz123".into(),
            top_k: 5,
        };
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/query")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: QueryResponse = serde_json::from_slice(
            &axum::body::to_bytes(resp.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert!(body.sources.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_stream_endpoint() {
        let app = routes().with_state(test_state());
        let req = QueryRequest {
            question: "test question".into(),
            top_k: 5,
        };
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/query/stream")
                    .method("POST")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_vec(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap(),
            "text/event-stream"
        );
    }

    #[tokio::test]
    async fn test_ingest_request_deserialization() {
        let json = r#"{"text": "hello world", "metadata": {"source": "test"}}"#;
        let req: IngestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.text, "hello world");
        assert_eq!(req.metadata.unwrap()["source"], "test");
    }

    #[tokio::test]
    async fn test_query_request_default_top_k() {
        let json = r#"{"question": "test"}"#;
        let req: QueryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.question, "test");
        assert_eq!(req.top_k, 5);
    }

    #[tokio::test]
    async fn test_query_response_serialization() {
        let resp = QueryResponse {
            answer: "test answer".into(),
            sources: vec![QuerySource {
                text: "source text".into(),
                score: 0.95,
                id: "abc-123".into(),
            }],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["answer"], "test answer");
        assert!((json["sources"][0]["score"].as_f64().unwrap() - 0.95).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_routes_are_registered() {
        let app = routes();
        let routes_str = format!("{:?}", app);
        assert!(routes_str.contains("/health"));
        assert!(routes_str.contains("/ingest"));
        assert!(routes_str.contains("/ingest/batch"));
        assert!(routes_str.contains("/query"));
        assert!(routes_str.contains("/query/stream"));
    }
}
