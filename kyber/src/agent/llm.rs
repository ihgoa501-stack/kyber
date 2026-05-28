use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

// ── Multi-provider LLM backend ──
// Controller and observer use independent backends (Separation Principle).
// Each backend has its own API key, provider, and model.

#[derive(Debug, Clone)]
pub enum Provider {
    Anthropic,
    OpenAI,
}

#[derive(Debug, Clone)]
pub struct Backend {
    pub name: String,
    pub provider: Provider,
    pub api_key: String,
    pub model: String,
}

impl Backend {
    /// Build from env vars with a prefix: KYBER_CONTROLLER_* or KYBER_OBSERVER_*
    pub fn from_env(role: &str) -> Result<Self> {
        let prefix = format!("KYBER_{}_", role.to_uppercase());

        let provider_str = std::env::var(format!("{}PROVIDER", prefix))
            .unwrap_or_else(|_| "anthropic".into());

        let provider = match provider_str.as_str() {
            "openai" => Provider::OpenAI,
            _ => Provider::Anthropic,
        };

        let api_key = std::env::var(format!("{}API_KEY", prefix))
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .map_err(|_| anyhow!("{}API_KEY 未设置", prefix))?;

        let default_model = match provider {
            Provider::Anthropic => "claude-sonnet-4-20250514",
            Provider::OpenAI => "gpt-4o",
        };
        let model = std::env::var(format!("{}MODEL", prefix))
            .unwrap_or_else(|_| default_model.into());

        Ok(Backend { name: role.into(), provider, api_key, model })
    }
}

/// Call any LLM backend.
pub async fn call(backend: &Backend, system: &str, prompt: &str) -> Result<String> {
    match backend.provider {
        Provider::Anthropic => call_anthropic(backend, system, prompt).await,
        Provider::OpenAI => call_openai(backend, system, prompt).await,
    }
}

// ── Anthropic ──

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

async fn call_anthropic(backend: &Backend, system: &str, prompt: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let request = AnthropicRequest {
        model: backend.model.clone(),
        max_tokens: 4096,
        system: system.into(),
        messages: vec![AnthropicMessage {
            role: "user".into(), content: prompt.into(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &backend.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("Anthropic {} ({}): {}", backend.model, status, text));
    }

    let body: AnthropicResponse = response.json().await?;
    Ok(body.content.first().map(|c| c.text.clone()).unwrap_or_default())
}

// ── OpenAI ──

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageContent,
}

#[derive(Deserialize)]
struct OpenAIMessageContent {
    content: String,
}

async fn call_openai(backend: &Backend, system: &str, prompt: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let messages = vec![
        OpenAIMessage { role: "system".into(), content: system.into() },
        OpenAIMessage { role: "user".into(), content: prompt.into() },
    ];

    let request = OpenAIRequest {
        model: backend.model.clone(),
        messages,
        max_tokens: 4096,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", backend.api_key))
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("OpenAI {} ({}): {}", backend.model, status, text));
    }

    let body: OpenAIResponse = response.json().await?;
    Ok(body.choices.first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_request_serialization() {
        let req = AnthropicRequest {
            model: "test".into(),
            max_tokens: 100,
            system: "be helpful".into(),
            messages: vec![AnthropicMessage { role: "user".into(), content: "hello".into() }],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"test\""));
    }

    #[test]
    fn test_openai_request_serialization() {
        let req = OpenAIRequest {
            model: "gpt-4o".into(),
            messages: vec![
                OpenAIMessage { role: "system".into(), content: "be helpful".into() },
                OpenAIMessage { role: "user".into(), content: "hello".into() },
            ],
            max_tokens: 100,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"gpt-4o\""));
    }
}
