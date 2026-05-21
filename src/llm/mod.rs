use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LlmClient {
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

#[allow(dead_code)]
#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
    finish_reason: Option<String>,
}

#[allow(dead_code)]
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
        match self.provider.as_str() {
            "openai" => {
                self.generate_openai(system_prompt, user_prompt, false)
                    .await
            }
            "anthropic" => {
                self.generate_anthropic(system_prompt, user_prompt, false)
                    .await
            }
            _ => {
                self.generate_openai(system_prompt, user_prompt, false)
                    .await
            }
        }
    }

    pub async fn generate_stream(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> anyhow::Result<impl tokio_stream::Stream<Item = Result<String, anyhow::Error>>> {
        match self.provider.as_str() {
            "openai" => self
                .generate_openai(system_prompt, user_prompt, true)
                .await
                .map(|_| tokio_stream::once(Ok("Streaming not fully implemented yet".to_string()))),
            _ => self
                .generate_openai(system_prompt, user_prompt, true)
                .await
                .map(|_| tokio_stream::once(Ok("Streaming not fully implemented yet".to_string()))),
        }
    }

    async fn generate_openai(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        _stream: bool,
    ) -> anyhow::Result<String> {
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
            stream: false,
            temperature: 0.1,
        };

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );
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

        let data: OpenAiResponse = resp.json().await?;
        Ok(data
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string())
    }

    async fn generate_anthropic(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
        _stream: bool,
    ) -> anyhow::Result<String> {
        // Anthropic API integration placeholder
        Ok("Anthropic support coming soon".to_string())
    }
}
