use anyhow::Context;
use reqwest::Client;
use serde_json::Value;

pub struct Reranker {
    client: Client,
    api_url: String,
    api_key: String,
}

impl Reranker {
    pub async fn new() -> anyhow::Result<Self> {
        let api_url = std::env::var("RERANKER_API_URL").unwrap_or_else(|_| {
            "https://api-inference.huggingface.co/models/cross-encoder/ms-marco-MiniLM-L-6-v2"
                .into()
        });
        let api_key = std::env::var("RERANKER_API_KEY")
            .or_else(|_| std::env::var("HF_API_KEY"))
            .unwrap_or_default();
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        tracing::info!("Reranker initialized, endpoint={}", api_url);
        Ok(Self {
            client,
            api_url,
            api_key,
        })
    }

    pub async fn rerank(
        &self,
        query: &str,
        texts: &[(String, String)],
    ) -> anyhow::Result<Vec<(String, String, f32)>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let pairs: Vec<String> = texts.iter().map(|(_, text)| text.clone()).collect();
        let body = serde_json::json!({
            "inputs": query,
            "inputs_pairs": pairs,
        });

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Content-Type", "application/json".parse().unwrap());
        if !self.api_key.is_empty() {
            headers.insert(
                "Authorization",
                format!("Bearer {}", self.api_key).parse().unwrap(),
            );
        }

        let resp = self
            .client
            .post(&self.api_url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .context("Reranker request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Reranker API error {}: {}", status, body);
        }

        let response: Value = resp
            .json()
            .await
            .context("Failed to parse reranker response")?;
        let scores = parse_scores(&response)?;

        anyhow::ensure!(
            scores.len() == texts.len(),
            "Reranker returned {} scores for {} documents",
            scores.len(),
            texts.len()
        );

        let mut results: Vec<(String, String, f32)> = texts
            .iter()
            .zip(scores.iter())
            .map(|((id, text), score)| (id.clone(), text.clone(), *score))
            .collect();

        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }
}

fn parse_scores(response: &Value) -> anyhow::Result<Vec<f32>> {
    match response {
        Value::Array(arr) if arr.is_empty() => Ok(Vec::new()),
        Value::Array(arr) => {
            if arr[0].is_f64() || arr[0].is_i64() || arr[0].is_u64() {
                arr.iter()
                    .map(|v| v.as_f64().map(|f| f as f32).context("Expected float score"))
                    .collect()
            } else if arr[0].is_array() {
                arr.iter()
                    .map(|inner| {
                        inner
                            .as_array()
                            .and_then(|inner_arr| inner_arr.first())
                            .and_then(|first| first.as_object())
                            .and_then(|obj| obj.get("score"))
                            .and_then(|s| s.as_f64())
                            .map(|s| s as f32)
                            .context("Expected classification format: [[{\"score\": ...}], ...]")
                    })
                    .collect()
            } else if arr[0].is_object() {
                arr.iter()
                    .map(|obj| {
                        obj.as_object()
                            .and_then(|m| m.get("score"))
                            .and_then(|s| s.as_f64())
                            .map(|s| s as f32)
                            .context("Expected object format: {\"score\": ...}")
                    })
                    .collect()
            } else {
                anyhow::bail!("Unexpected array element type: {}", arr[0])
            }
        }
        _ => anyhow::bail!("Unexpected reranker response: {}", response),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_scores_direct() {
        let v = json!([0.9, 0.1, 0.5]);
        let scores = parse_scores(&v).unwrap();
        assert_eq!(scores, vec![0.9, 0.1, 0.5]);
    }

    #[test]
    fn test_parse_scores_classification() {
        let v = json!([
            [{"label": "LABEL_0", "score": 0.95}],
            [{"label": "LABEL_0", "score": 0.30}]
        ]);
        let scores = parse_scores(&v).unwrap();
        assert!((scores[0] - 0.95).abs() < 0.01);
        assert!((scores[1] - 0.30).abs() < 0.01);
    }

    #[test]
    fn test_parse_scores_objects() {
        let v = json!([
            {"score": 0.85},
            {"score": 0.42}
        ]);
        let scores = parse_scores(&v).unwrap();
        assert!((scores[0] - 0.85).abs() < 0.01);
        assert!((scores[1] - 0.42).abs() < 0.01);
    }

    #[test]
    fn test_parse_scores_empty() {
        let v = json!([]);
        let scores = parse_scores(&v).unwrap();
        assert!(scores.is_empty());
    }

    #[test]
    fn test_rerank_sorting() {
        let _reranker = Reranker {
            client: Client::new(),
            api_url: String::new(),
            api_key: String::new(),
        };
        let texts = vec![
            ("a".into(), "doc a".into()),
            ("b".into(), "doc b".into()),
            ("c".into(), "doc c".into()),
        ];
        let scores = vec![0.3, 0.9, 0.1];
        let mut results: Vec<(String, String, f32)> = texts
            .into_iter()
            .zip(scores.into_iter())
            .map(|((id, text), score)| (id, text, score))
            .collect();
        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(results[0].0, "b");
        assert_eq!(results[1].0, "a");
        assert_eq!(results[2].0, "c");
    }
}
