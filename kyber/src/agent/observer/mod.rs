/// Kyber Observer — orchestration layer.
///
/// Architecture:
///   Primary:   SignalFusion (statistical) → computes confidence, no LLM
///   Secondary: LLM backend (if configured) → qualitative summary + issues
///
/// Separation Principle: controller decides, observer evaluates.
/// The observer's confidence comes from hard signals, not AI self-assessment.
pub mod signal_fusion;

use std::time::{SystemTime, UNIX_EPOCH};
use super::llm::Backend;
use signal_fusion::SignalFusion;

pub struct Observer {
    pub confidence_threshold: f64,
    pub backend: Backend,
    pub fusion: SignalFusion,
    context: Vec<String>,
    llm_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct Observation {
    pub confidence: f64,
    pub summary: String,
    pub issues: Vec<String>,
    pub timestamp: u64,
    /// Per-signal breakdown for debugging
    pub breakdown: Vec<(String, f64, f64)>,
}

impl Observer {
    pub fn new(confidence_threshold: f64, backend: Backend, task: &str) -> Self {
        Observer {
            confidence_threshold,
            backend,
            fusion: SignalFusion::new(task.into()),
            context: Vec::new(),
            llm_enabled: true,
        }
    }

    /// Disable LLM observer — purely statistical mode.
    pub fn without_llm(mut self) -> Self {
        self.llm_enabled = false;
        self
    }

    /// Record a step's outcome for signal computation.
    pub fn record_step(&mut self, success: bool, action_kind: &str, output_len: usize) {
        let is_dangerous = matches!(action_kind, "delete" | "execute" | "git_push" | "write");
        self.fusion.record(success, action_kind, output_len, is_dangerous);
    }

    /// Add unstructured context (for LLM observer).
    pub fn add_context(&mut self, entry: String) {
        self.context.push(entry);
        if self.context.len() > 50 {
            self.context.remove(0);
        }
    }

    pub fn context_str(&self) -> String {
        self.context.join("\n")
    }

    /// Observe current state.
    ///
    /// 1. Signal fusion computes confidence from hard stats (always runs, no API needed)
    /// 2. LLM provides qualitative interpretation (runs in parallel, best-effort)
    pub async fn observe(&self) -> Observation {
        // 1. Statistical confidence — always available
        let signal_report = self.fusion.evaluate();

        // 2. Qualitative LLM interpretation — best-effort, runs in parallel
        let (qual_summary, qual_issues) = if self.llm_enabled {
            self.llm_observe(&signal_report).await
        } else {
            (format!("统计置信度: {:.2}", signal_report.confidence), vec![])
        };

        Observation {
            confidence: signal_report.confidence,
            summary: qual_summary,
            issues: qual_issues,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH).unwrap().as_secs(),
            breakdown: signal_report.breakdown,
        }
    }

    /// Secondary: ask LLM for qualitative interpretation.
    /// Does NOT compute confidence — only summary and issue list.
    async fn llm_observe(&self, signals: &signal_fusion::SignalReport) -> (String, Vec<String>) {
        let prompt = format!(
            "信号报告:\n\
             - 工具成功率: {:.2}\n\
             - 非重复度:   {:.2}\n\
             - 步效比:     {:.2}\n\
             - 正常操作度: {:.2}\n\
             - 综合置信度: {:.2}\n\n\
             上下文:\n{}",
            signals.tool_success_rate,
            signals.repetition_score,
            signals.efficiency_score,
            signals.anomaly_score,
            signals.confidence,
            self.context_str(),
        );

        let sys = "你是观测器。基于统计信号和上下文，输出 JSON: {\"summary\": \"一句话诊断\", \"issues\": [\"问题1\"]}。不要提置信度数字，只说 Agent 当前状态。";

        match super::llm::call(&self.backend, sys, &prompt).await {
            Ok(text) => {
                if let Some(json_str) = extract_json(&text) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
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
                        return (summary, issues);
                    }
                }
                (text, vec![])
            }
            Err(_) => {
                (format!("LLM 观测不可用。统计置信度: {:.2}", signals.confidence), vec!["LLM 观测器离线".into()])
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
