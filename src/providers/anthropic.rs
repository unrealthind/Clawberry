use crate::providers::traits::{Provider, TokenUsage, UsageTracker};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub struct AnthropicProvider {
    credential: Option<String>,
    base_url: String,
    client: Client,
    usage_tracker: Option<UsageTracker>,
    output_limit: u32,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

// Newer Claude models reject sampling overrides. Retain configured temperature
// only for known legacy generations; unknown future identifiers omit it safely.
fn supported_temperature(model: &str, temperature: f64) -> Option<f64> {
    let parts: Vec<_> = model.split('-').collect();
    let legacy = model.starts_with("claude-2")
        || model.starts_with("claude-3-")
        || model.starts_with("claude-3-5-")
        || model.starts_with("claude-3-7-");
    let fourth_generation = parts.first() == Some(&"claude")
        && matches!(parts.get(1), Some(&"opus" | &"sonnet" | &"haiku"))
        && parts.get(2) == Some(&"4")
        && parts.get(3).is_some_and(|part| {
            part.parse::<u32>()
                .is_ok_and(|version| version <= 6 || version >= 20_000_000)
        });
    (legacy || fourth_generation).then_some(temperature)
}

/// SSE event data for Anthropic streaming
#[derive(Debug, Deserialize)]
struct StreamContentDelta {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    delta: Option<StreamContentDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamMessageDelta {
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamMessageDeltaEvent {
    #[serde(default)]
    delta: Option<serde_json::Value>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text {
        text: String,
    },
    #[serde(other)]
    Other,
}

impl ContentBlock {
    fn into_text(self) -> Option<String> {
        match self {
            Self::Text { text } => Some(text),
            Self::Other => None,
        }
    }
}

fn response_text(content: Vec<ContentBlock>) -> anyhow::Result<String> {
    let text = content
        .into_iter()
        .filter_map(ContentBlock::into_text)
        .collect::<String>();
    if text.is_empty() {
        anyhow::bail!("Anthropic response contained no text answer");
    }
    Ok(text)
}

impl AnthropicProvider {
    pub fn new(api_key: Option<&str>) -> Self {
        Self::with_base_url(api_key, None)
    }

    pub fn with_base_url(api_key: Option<&str>, base_url: Option<&str>) -> Self {
        let base_url = base_url
            .map_or("https://api.anthropic.com", |u| u.trim_end_matches('/'))
            .to_string();
        Self {
            credential: api_key
                .map(str::trim)
                .filter(|k| !k.is_empty())
                .map(ToString::to_string),
            base_url,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| Client::new()),
            usage_tracker: None,
            output_limit: 4096,
        }
    }

    fn is_setup_token(token: &str) -> bool {
        token.starts_with("sk-ant-oat01-")
    }
}

#[async_trait]
#[allow(clippy::too_many_lines)] // Keep streaming protocol handling in one implementation.
impl Provider for AnthropicProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "Anthropic credentials not set. Set ANTHROPIC_API_KEY or ANTHROPIC_OAUTH_TOKEN (setup-token)."
            )
        })?;

        let request = ChatRequest {
            model: model.to_string(),
            max_tokens: self.output_limit,
            system: system_prompt.map(ToString::to_string),
            messages: vec![Message {
                role: "user".to_string(),
                content: message.to_string(),
            }],
            temperature: supported_temperature(model, temperature),
            stream: None,
        };

        let mut request = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request);

        if Self::is_setup_token(credential) {
            request = request.header("Authorization", format!("Bearer {credential}"));
        } else {
            request = request.header("x-api-key", credential);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(super::api_error("Anthropic", response).await);
        }

        let chat_response: ChatResponse = response.json().await?;

        if let (Some(tracker), Some(u)) = (&self.usage_tracker, &chat_response.usage) {
            tracker.add(&TokenUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            });
        }

        response_text(chat_response.content)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    async fn chat_with_history_stream(
        &self,
        history: &[crate::providers::ChatMessage],
        model: &str,
        temperature: f64,
        token_tx: mpsc::Sender<String>,
    ) -> anyhow::Result<String> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Anthropic credentials not set."))?;

        // Separate system prompt from messages
        let mut system_prompt: Option<String> = None;
        let mut messages = Vec::new();
        for msg in history {
            if msg.role == "system" {
                system_prompt = Some(msg.content.clone());
            } else {
                messages.push(Message {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                });
            }
        }

        let request_body = ChatRequest {
            model: model.to_string(),
            max_tokens: self.output_limit,
            system: system_prompt,
            messages,
            temperature: supported_temperature(model, temperature),
            stream: Some(true),
        };

        let mut req = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body);

        if Self::is_setup_token(credential) {
            req = req.header("Authorization", format!("Bearer {credential}"));
        } else {
            req = req.header("x-api-key", credential);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(super::api_error("Anthropic", response).await);
        }

        let mut full_text = String::new();
        let mut buffer = Vec::new();
        let mut input_tokens = None;
        let mut output_tokens = None;
        let mut completed = false;
        let mut stream = response;
        'chunks: while let Some(chunk) = stream.chunk().await? {
            buffer.extend_from_slice(&chunk);
            if buffer.len() > 1_048_576 {
                anyhow::bail!("Anthropic stream event exceeds limit");
            }
            while let Some(end) = buffer.iter().position(|byte| *byte == b'\n') {
                let line = std::str::from_utf8(&buffer[..end])?
                    .trim_end_matches('\r')
                    .to_owned();
                buffer.drain(..=end);
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim_start();
                if data.is_empty() {
                    continue;
                }
                let event: serde_json::Value = serde_json::from_str(data)?;
                match event
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                {
                    "content_block_delta" => {
                        if let Some(text) = event
                            .pointer("/delta/text")
                            .and_then(serde_json::Value::as_str)
                        {
                            if full_text.len().saturating_add(text.len()) > 1_048_576 {
                                anyhow::bail!("Anthropic response exceeds limit");
                            }
                            full_text.push_str(text);
                            token_tx
                                .send(text.to_owned())
                                .await
                                .map_err(|_| anyhow::anyhow!("Chat stream disconnected"))?;
                        }
                    }
                    "message_start" => {
                        input_tokens = event
                            .pointer("/message/usage/input_tokens")
                            .and_then(serde_json::Value::as_u64);
                    }
                    "message_delta" => {
                        output_tokens = event
                            .pointer("/usage/output_tokens")
                            .and_then(serde_json::Value::as_u64)
                            .or(output_tokens);
                    }
                    "message_stop" => {
                        completed = true;
                        break 'chunks;
                    }
                    "error" => anyhow::bail!("Anthropic reported a streaming error"),
                    _ => {}
                }
            }
        }
        if !completed {
            anyhow::bail!("Anthropic stream ended without completion confirmation");
        }
        if let (Some(input), Some(output), Some(tracker)) =
            (input_tokens, output_tokens, &self.usage_tracker)
        {
            tracker.add(&TokenUsage {
                prompt_tokens: input,
                completion_tokens: output,
                total_tokens: input.saturating_add(output),
            });
        }

        Ok(full_text)
    }

    fn set_output_token_limit(&mut self, limit: u32) -> anyhow::Result<()> {
        self.output_limit = limit;
        Ok(())
    }

    async fn chat_with_history(
        &self,
        history: &[crate::providers::ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let credential = self
            .credential
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Anthropic credentials not set"))?;
        let system = history
            .iter()
            .filter(|m| m.role == "system")
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let messages = history
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| Message {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        let body = ChatRequest {
            model: model.into(),
            max_tokens: self.output_limit,
            system: Some(system),
            messages,
            temperature: supported_temperature(model, temperature),
            stream: None,
        };
        let mut request = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        request = if Self::is_setup_token(credential) {
            request.header("Authorization", format!("Bearer {credential}"))
        } else {
            request.header("x-api-key", credential)
        };
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(super::api_error("Anthropic", response).await);
        }
        let result: ChatResponse = response.json().await?;
        if let (Some(tracker), Some(usage)) = (&self.usage_tracker, &result.usage) {
            tracker.add(&TokenUsage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
            });
        }
        response_text(result.content)
    }

    fn set_usage_tracker(&mut self, tracker: UsageTracker) {
        self.usage_tracker = Some(tracker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_key() {
        let p = AnthropicProvider::new(Some("sk-ant-test123"));
        assert!(p.credential.is_some());
        assert_eq!(p.credential.as_deref(), Some("sk-ant-test123"));
        assert_eq!(p.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn creates_without_key() {
        let p = AnthropicProvider::new(None);
        assert!(p.credential.is_none());
        assert_eq!(p.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn creates_with_empty_key() {
        let p = AnthropicProvider::new(Some(""));
        assert!(p.credential.is_none());
    }

    #[test]
    fn creates_with_whitespace_key() {
        let p = AnthropicProvider::new(Some("  sk-ant-test123  "));
        assert!(p.credential.is_some());
        assert_eq!(p.credential.as_deref(), Some("sk-ant-test123"));
    }

    #[test]
    fn creates_with_custom_base_url() {
        let p =
            AnthropicProvider::with_base_url(Some("sk-ant-test"), Some("https://api.example.com"));
        assert_eq!(p.base_url, "https://api.example.com");
        assert_eq!(p.credential.as_deref(), Some("sk-ant-test"));
    }

    #[test]
    fn custom_base_url_trims_trailing_slash() {
        let p = AnthropicProvider::with_base_url(None, Some("https://api.example.com/"));
        assert_eq!(p.base_url, "https://api.example.com");
    }

    #[test]
    fn default_base_url_when_none_provided() {
        let p = AnthropicProvider::with_base_url(None, None);
        assert_eq!(p.base_url, "https://api.anthropic.com");
    }

    #[tokio::test]
    async fn chat_fails_without_key() {
        let p = AnthropicProvider::new(None);
        let result = p
            .chat_with_system(None, "hello", "claude-3-opus", 0.7)
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("credentials not set"),
            "Expected key error, got: {err}"
        );
    }

    #[test]
    fn setup_token_detection_works() {
        assert!(AnthropicProvider::is_setup_token("sk-ant-oat01-abcdef"));
        assert!(!AnthropicProvider::is_setup_token("sk-ant-api-key"));
    }

    #[tokio::test]
    async fn chat_with_system_fails_without_key() {
        let p = AnthropicProvider::new(None);
        let result = p
            .chat_with_system(Some("You are TinyClaw"), "hello", "claude-3-opus", 0.7)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn chat_request_serializes_without_system() {
        let req = ChatRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 4096,
            system: None,
            messages: vec![Message {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            temperature: Some(0.7),
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(
            !json.contains("system"),
            "system field should be skipped when None"
        );
        assert!(json.contains("claude-3-opus"));
        assert!(json.contains("hello"));
    }

    #[test]
    fn chat_request_serializes_with_system() {
        let req = ChatRequest {
            model: "claude-3-opus".to_string(),
            max_tokens: 4096,
            system: Some("You are TinyClaw".to_string()),
            messages: vec![Message {
                role: "user".to_string(),
                content: "hello".to_string(),
            }],
            temperature: Some(0.7),
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"system\":\"You are TinyClaw\""));
    }

    #[test]
    fn chat_response_deserializes() {
        let json = r#"{"content":[{"type":"text","text":"Hello there!"}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert_eq!(response_text(resp.content).unwrap(), "Hello there!");
    }

    #[test]
    fn chat_response_empty_content() {
        let json = r#"{"content":[]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.content.is_empty());
    }

    #[test]
    fn chat_response_multiple_blocks() {
        let json =
            r#"{"content":[{"type":"text","text":"First"},{"type":"text","text":"Second"}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content.len(), 2);
        assert_eq!(response_text(resp.content).unwrap(), "FirstSecond");
    }

    #[test]
    fn mixed_response_blocks_return_only_final_text() {
        let response: ChatResponse = serde_json::from_value(serde_json::json!({
            "content": [
                {"type":"thinking","thinking":"private","signature":"opaque"},
                {"type":"redacted_thinking","data":"opaque"},
                {"type":"text","text":"answer"},
                {"type":"tool_use","id":"test","name":"test","input":{}}
            ], "usage":{"input_tokens":10,"output_tokens":20}
        }))
        .unwrap();
        assert_eq!(response_text(response.content).unwrap(), "answer");
        let no_text: ChatResponse = serde_json::from_value(serde_json::json!({
            "content":[{"type":"thinking","signature":"opaque"}]
        }))
        .unwrap();
        assert!(response_text(no_text.content).is_err());
        assert!(serde_json::from_value::<ChatResponse>(serde_json::json!({
            "content":[{"type":"text"}]
        }))
        .is_err());
    }

    #[test]
    fn modern_models_omit_temperature_from_wire_request() {
        for model in [
            "claude-opus-4-7",
            "claude-sonnet-5",
            "claude-opus-5-5",
            "claude-mythos-preview",
        ] {
            let request = ChatRequest {
                model: model.into(),
                max_tokens: 128,
                system: None,
                messages: vec![],
                temperature: supported_temperature(model, 0.7),
                stream: None,
            };
            assert!(serde_json::to_value(request)
                .unwrap()
                .get("temperature")
                .is_none());
        }
        for model in [
            "claude-3-5-sonnet-20241022",
            "claude-opus-4-6",
            "claude-sonnet-4-20250514",
        ] {
            assert_eq!(supported_temperature(model, 0.7), Some(0.7));
        }
    }

    #[test]
    fn temperature_range_serializes() {
        for temp in [0.0, 0.5, 1.0, 2.0] {
            let req = ChatRequest {
                model: "claude-3-opus".to_string(),
                max_tokens: 4096,
                system: None,
                messages: vec![],
                temperature: Some(temp),
                stream: None,
            };
            let json = serde_json::to_string(&req).unwrap();
            assert!(json.contains(&format!("{temp}")));
        }
    }
}
