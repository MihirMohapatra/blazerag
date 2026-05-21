#[cfg(feature = "onnx")]
mod onnx;
#[cfg(feature = "onnx")]
pub use onnx::OrtEmbedder;

mod http;
pub use http::HttpEmbedder;

const EMBEDDING_DIM: usize = 384;

pub trait EmbedderTrait: Send + Sync {
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>>;
    fn embedding_dim(&self) -> usize {
        EMBEDDING_DIM
    }
}

pub enum Embedder {
    Http(HttpEmbedder),
    #[cfg(feature = "onnx")]
    Onnx(OrtEmbedder),
}

impl Embedder {
    pub async fn new() -> anyhow::Result<Self> {
        let backend = std::env::var("EMBEDDING_BACKEND").unwrap_or_else(|_| "http".into());

        match backend.as_str() {
            #[cfg(feature = "onnx")]
            "onnx" => {
                let path = std::env::var("ONNX_MODEL_PATH")
                    .unwrap_or_else(|_| "./models/all-MiniLM-L6-v2.onnx".into());
                OrtEmbedder::new(&path).await.map(Embedder::Onnx)
            }
            _ => HttpEmbedder::new().await.map(Embedder::Http),
        }
    }

    pub fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        match self {
            Embedder::Http(h) => h.embed(texts),
            #[cfg(feature = "onnx")]
            Embedder::Onnx(o) => o.embed(texts),
        }
    }

    pub fn embedding_dim(&self) -> usize {
        match self {
            Embedder::Http(h) => h.embedding_dim(),
            #[cfg(feature = "onnx")]
            Embedder::Onnx(o) => o.embedding_dim(),
        }
    }
}
