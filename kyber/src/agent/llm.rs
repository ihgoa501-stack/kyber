use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct MessagesRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

/// Call the Anthropic Messages API.
/// Returns the response text.
pub async fn call(prompt: &str, system: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow!("ANTHROPIC_API_KEY 未设置"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let request = MessagesRequest {
        model: "claude-sonnet-4-20250514".into(),
        max_tokens: 4096,
        system: system.into(),
        messages: vec![Message {
            role: "user".into(),
            content: prompt.into(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow!("API 错误 ({}): {}", status, text));
    }

    let body: MessagesResponse = response.json().await?;
    let text = body.content.first()
        .map(|c| c.text.clone())
        .unwrap_or_default();

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = MessagesRequest {
            model: "test".into(),
            max_tokens: 100,
            system: "be helpful".into(),
            messages: vec![Message { role: "user".into(), content: "hello".into() }],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\""));
        assert!(json.contains("\"test\""));
    }
}
