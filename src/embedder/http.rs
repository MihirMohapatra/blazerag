use super::EmbedderTrait;

pub struct HttpEmbedder {
    client: reqwest::Client,
    api_url: String,
    api_key: String,
    dim: usize,
}

impl HttpEmbedder {
    pub async fn new() -> anyhow::Result<Self> {
        let api_url = std::env::var("EMBEDDING_API_URL")
            .unwrap_or_else(|_| "https://api-inference.huggingface.co/pipeline/feature-extraction/sentence-transformers/all-MiniLM-L6-v2".into());
        let api_key = std::env::var("EMBEDDING_API_KEY").unwrap_or_default();
        let dim: usize = std::env::var("EMBEDDING_DIM")
            .unwrap_or_else(|_| "384".into())
            .parse()
            .unwrap_or(384);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;

        tracing::info!(
            "HTTP embedder initialized, dim={}, endpoint={}",
            dim,
            api_url
        );

        Ok(Self {
            client,
            api_url,
            api_key,
            dim,
        })
    }
}

impl EmbedderTrait for HttpEmbedder {
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let rt = tokio::runtime::Handle::current();
        let this = self;
        rt.block_on(async {
            let texts_batch: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("Content-Type", "application/json".parse().unwrap());
            if !this.api_key.is_empty() {
                headers.insert("Authorization", format!("Bearer {}", this.api_key).parse().unwrap());
            }

            let resp = this.client
                .post(&this.api_url)
                .headers(headers)
                .json(&texts_batch)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("HTTP embedding request failed: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                // Return random embeddings as fallback for dev/testing
                if status.as_u16() == 503 || status.as_u16() == 401 {
                    tracing::warn!("Embedding API returned {} (model may be loading or auth issue). Using random embeddings as fallback.", status);
                    return Ok(texts.iter().map(|t| self::random_embedding(t, this.dim)).collect());
                }
                anyhow::bail!("Embedding API error {}: {}", status, body);
            }

            let embedding: Vec<Vec<f32>> = resp.json().await
                .map_err(|e| anyhow::anyhow!("Failed to parse embedding response: {}", e))?;

            if embedding.is_empty() {
                anyhow::bail!("Embedding API returned empty result");
            }

            Ok(embedding)
        })
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

fn random_embedding(text: &str, dim: usize) -> Vec<f32> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    let seed = hasher.finish();

    let mut vec: Vec<f32> = (0..dim)
        .map(|i| {
            let x = ((i as u64).wrapping_mul(seed) as f64).sin() * 10000.0;
            ((x - x.floor()) * 2.0 - 1.0) as f32
        })
        .collect();
    let norm: f32 = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
    vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_embedding() {
        let emb = random_embedding("hello", 384);
        assert_eq!(emb.len(), 384);
        let norm: f32 = emb.iter().map(|v| v * v).sum();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_deterministic() {
        let a = random_embedding("hello", 384);
        let b = random_embedding("hello", 384);
        assert_eq!(a, b);
        let c = random_embedding("world", 384);
        assert_ne!(a, c);
    }
}
