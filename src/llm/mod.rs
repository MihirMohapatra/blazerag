use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Debug, Clone)]
pub struct LlmClient {
    #[allow(dead_code)]
    provider: String,
    api_key: String,
    model: String,
    endpoint: String,
    client: Client,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: Option<String>,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}

impl LlmClient {
    pub fn new(provider: &str, api_key: &str, model: &str, endpoint: &str) -> Self {
        Self {
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            endpoint: endpoint.to_string(),
            client: Client::new(),
        }
    }

    pub async fn generate(&self, system_prompt: &str, user_prompt: &str) -> anyhow::Result<String> {
        let resp = self.send_request(system_prompt, user_prompt, false).await?;
        let data: OpenAiResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string())
    }

    pub async fn generate_stream(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<impl tokio_stream::Stream<Item = Result<String, anyhow::Error>>> {
        let (tx, rx) = mpsc::channel::<Result<String, anyhow::Error>>(64);

        let resp = self.send_request(system_prompt, user_prompt, true).await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, body);
        }

        let mut stream = resp.bytes_stream();
        tokio::spawn(async move {
            let mut buffer = String::new();
            while let Some(chunk_result) = stream.next().await {
                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("Stream error: {}", e))).await;
                        return;
                    }
                };
                buffer.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(newline_idx) = buffer.find('\n') {
                    let line = buffer[..newline_idx].trim_end_matches('\r').to_string();
                    buffer = buffer[newline_idx + 1..].to_string();
                    if line.is_empty() {
                        continue;
                    }
                    if line == "data: [DONE]" {
                        return;
                    }
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    if !content.is_empty()
                                        && tx.send(Ok(content.clone())).await.is_err()
                                    {
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(ReceiverStream::new(rx))
    }

    async fn send_request(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        stream: bool,
    ) -> anyhow::Result<reqwest::Response> {
        let body = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".into(),
                    content: system_prompt.to_string(),
                },
                Message {
                    role: "user".into(),
                    content: user_prompt.to_string(),
                },
            ],
            stream,
            temperature: 0.1,
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
        if stream {
            headers.insert(
                reqwest::header::ACCEPT,
                "text/event-stream".parse().unwrap(),
            );
        }
        if !self.api_key.is_empty() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.api_key).parse().unwrap(),
            );
        }

        let resp = self
            .client
            .post(&self.endpoint)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        Ok(resp)
    }
}
