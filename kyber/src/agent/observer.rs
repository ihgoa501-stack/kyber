use std::time::{SystemTime, UNIX_EPOCH};
use super::llm::Backend;

pub struct Observer {
    pub confidence_threshold: f64,
    pub backend: Backend,
    context: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub confidence: f64,
    pub summary: String,
    pub issues: Vec<String>,
    pub timestamp: u64,
}

impl Observer {
    pub fn new(confidence_threshold: f64, backend: Backend) -> Self {
        Observer {
            confidence_threshold,
            backend,
            context: Vec::new(),
        }
    }

    pub fn add_context(&mut self, entry: String) {
        self.context.push(entry);
        if self.context.len() > 50 {
            self.context.remove(0);
        }
    }

    pub fn context_str(&self) -> String {
        self.context.join("\n")
    }

    /// Observe by asking LLM to assess current state.
    /// If LLM unavailable, falls back to a default confidence estimate.
    pub async fn observe(&self) -> Observation {
        let prompt = if self.context.is_empty() {
            format!("当前没有历史上下文。请评估初始状态。")
        } else {
            format!("当前上下文:\n{}\n\n请评估当前进展和置信度。", self.context_str())
        };

        let sys = "你是一个 AI Agent 的观测器。评估当前状态，输出 JSON: {\"confidence\": 0.0-1.0, \"summary\": \"一句话总结\", \"issues\": [\"问题1\"]}";

        match super::llm::call(&self.backend, sys, &prompt).await {
            Ok(text) => {
                // Try to parse JSON from response
                if let Some(json_str) = extract_json(&text) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        let confidence = parsed.get("confidence")
                            .and_then(|c| c.as_f64())
                            .unwrap_or(0.5)
                            .clamp(0.0, 1.0);
                        let summary = parsed.get("summary")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        let issues: Vec<String> = parsed.get("issues")
                            .and_then(|i| i.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect())
                            .unwrap_or_default();
                        return Observation {
                            confidence,
                            summary,
                            issues,
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        };
                    }
                }
                // Fallback: estimate confidence from text
                let confidence = if text.contains("confident") || text.contains("sure") { 0.7 }
                    else if text.contains("uncertain") || text.contains("unsure") { 0.3 }
                    else { 0.5 };
                Observation { confidence, summary: text, issues: vec![], timestamp: now() }
            }
            Err(_) => {
                Observation { confidence: 0.5, summary: "LLM 不可用".into(), issues: vec!["LLM 连接失败".into()], timestamp: now() }
            }
        }
    }
}

fn extract_json(text: &str) -> Option<String> {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Some(text[start..=end].to_string());
        }
    }
    None
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}
