use std::path::Path;

use super::EmbedderTrait;
use ort::session::Session;
use ort::value::Tensor;

pub struct OrtEmbedder {
    session: Session,
    dim: usize,
}

impl OrtEmbedder {
    pub async fn new(model_path: &str) -> anyhow::Result<Self> {
        let path = Path::new(model_path);

        if !path.exists() {
            tracing::warn!("ONNX model not found at {}. Attempting download...", model_path);
            Self::download_model(model_path).await?;
        }

        let session = Session::builder()?
            .commit_from_file(model_path)?;

        Ok(Self { session, dim: 384 })
    }

    async fn download_model(path: &str) -> anyhow::Result<()> {
        let p = Path::new(path);
        if let Some(parent) = p.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let url = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx?download=1";
        tracing::info!("Downloading model from HuggingFace...");
        let resp = reqwest::get(url).await?;
        let bytes = resp.bytes().await?;
        tokio::fs::write(path, &bytes).await?;
        tracing::info!("Model saved to {}", path);
        Ok(())
    }
}

impl EmbedderTrait for OrtEmbedder {
    fn embed(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed_single(t)).collect()
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

impl OrtEmbedder {
    fn embed_single(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let max_len = 256;
        let truncated: Vec<&str> = tokens.iter().take(max_len).copied().collect();
        let seq_len = truncated.len().max(1);

        let mut input_ids: Vec<i64> = truncated.iter().map(|t| {
            (t.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64)) % 30522) as i64
        }).collect();
        while input_ids.len() < seq_len {
            input_ids.push(0);
        }

        let attention_mask = vec![1i64; seq_len];
        let token_type_ids = vec![0i64; seq_len];

        let input_tensor = Tensor::from_array((
            vec![1usize, seq_len],
            input_ids.into_boxed_slice(),
        ))?;

        let mask_tensor = Tensor::from_array((
            vec![1usize, seq_len],
            attention_mask.into_boxed_slice(),
        ))?;

        let tt_tensor = Tensor::from_array((
            vec![1usize, seq_len],
            token_type_ids.into_boxed_slice(),
        ))?;

        let outputs = self.session.run(
            ort::inputs![
                "input_ids" => input_tensor,
                "attention_mask" => mask_tensor,
                "token_type_ids" => tt_tensor,
            ]
        )?;

        let (_shape, data) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()?;

        let hidden_dim = 384;
        let mut embedding = vec![0.0f32; hidden_dim];

        for i in 0..seq_len {
            let offset = i * hidden_dim;
            for j in 0..hidden_dim {
                embedding[j] += data[offset + j];
            }
        }
        for j in 0..hidden_dim {
            embedding[j] /= seq_len as f32;
        }

        let norm: f32 = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in embedding.iter_mut() {
                *v /= norm;
            }
        }

        Ok(embedding)
    }
}
