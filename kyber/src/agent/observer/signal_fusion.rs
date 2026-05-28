/// Statistical Signal Fusion Engine
///
/// Computes agent confidence from 5 independent signals — no LLM involved.
/// Each signal returns 0.0–1.0 where 1.0 = healthy. Weighted sum → confidence.
///
/// This is the primary observer. The LLM observer is secondary (qualitative only).
use std::collections::VecDeque;

/// A single step record for signal computation
#[derive(Debug, Clone)]
pub struct StepRecord {
    pub success: bool,
    pub action_kind: String,
    pub output_len: usize,
    /// Was this action dangerous (delete, execute, git_push, etc.)?
    pub is_dangerous: bool,
}

#[derive(Debug, Clone)]
pub struct SignalReport {
    pub confidence: f64,
    pub tool_success_rate: f64,
    pub repetition_score: f64,  // 1.0 = no repetition, 0.0 = high repetition
    pub efficiency_score: f64,  // 1.0 = efficient, 0.0 = wasting steps
    pub anomaly_score: f64,     // 1.0 = normal, 0.0 = anomalous
    /// Per-signal breakdown for debugging
    pub breakdown: Vec<(String, f64, f64)>, // (name, score, weight)
}

pub struct SignalFusion {
    steps: VecDeque<StepRecord>,
    window: usize,
    // Repetition tracker: stores hashes of recent actions
    recent_actions: VecDeque<String>,
    // Weight config
    w_tool_success: f64,
    w_repetition: f64,
    w_efficiency: f64,
    w_anomaly: f64,
    // The original task for drift comparison
    task: String,
    // Total productive output bytes
    total_output: usize,
}

impl SignalFusion {
    pub fn new(task: String) -> Self {
        SignalFusion {
            steps: VecDeque::new(),
            window: 15,
            recent_actions: VecDeque::new(),
            w_tool_success: 0.30,
            w_repetition: 0.25,
            w_efficiency: 0.25,
            w_anomaly: 0.20,
            task,
            total_output: 0,
        }
    }

    /// Record a step for future signal computation.
    pub fn record(&mut self, success: bool, action_kind: &str, output_len: usize, is_dangerous: bool) {
        self.steps.push_back(StepRecord { success, action_kind: action_kind.into(), output_len, is_dangerous });
        self.recent_actions.push_back(action_kind.into());
        self.total_output += output_len;

        // Trim to window
        if self.steps.len() > self.window {
            self.steps.pop_front();
        }
        if self.recent_actions.len() > self.window {
            self.recent_actions.pop_front();
        }
    }

    /// Compute confidence from all signals.
    /// Returns (confidence, per_signal_breakdown).
    pub fn evaluate(&self) -> SignalReport {
        let tool_success = self.signal_tool_success();
        let repetition = self.signal_repetition();
        let efficiency = self.signal_efficiency();
        let anomaly = self.signal_anomaly();

        let confidence = tool_success * self.w_tool_success
            + repetition * self.w_repetition
            + efficiency * self.w_efficiency
            + anomaly * self.w_anomaly;

        let confidence = confidence.clamp(0.0, 1.0);

        let breakdown = vec![
            ("工具成功率".into(), tool_success, self.w_tool_success),
            ("非重复度".into(), repetition, self.w_repetition),
            ("步效比".into(), efficiency, self.w_efficiency),
            ("正常操作密度".into(), anomaly, self.w_anomaly),
        ];

        SignalReport { confidence, tool_success_rate: tool_success, repetition_score: repetition, efficiency_score: efficiency, anomaly_score: anomaly, breakdown }
    }

    /// Signal 1: Tool success rate in recent window.
    /// Higher = more successful tool calls.
    fn signal_tool_success(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.5;
        }
        let count = self.steps.len();
        let successes = self.steps.iter().filter(|s| s.success).count();
        let raw = successes as f64 / count as f64;

        // Apply trend weight: recent failures hurt more
        let recent_5: Vec<bool> = self.steps.iter().rev().take(5).map(|s| s.success).collect();
        let recent_rate = recent_5.iter().filter(|&&s| s).count() as f64 / recent_5.len() as f64;

        // Blend: 60% recent, 40% full window
        (recent_rate * 0.6 + raw * 0.4).clamp(0.0, 1.0)
    }

    /// Signal 2: Repetition check.
    /// If the agent keeps doing the same action, it's stuck in a loop.
    fn signal_repetition(&self) -> f64 {
        if self.recent_actions.len() < 4 {
            return 0.9;
        }

        // Only track "concerning" actions for repetition.
        // Read/write are naturally repetitive (bulk operations).
        let concerning: Vec<&String> = self.recent_actions.iter()
            .filter(|a| matches!(a.as_str(), "think" | "execute" | "delete" | "browser"))
            .collect();

        if concerning.len() < 3 {
            return 1.0; // not enough concerning actions to judge
        }

        // Check consecutive repeats among concerning actions
        let mut max_consecutive = 1;
        let mut current = 1;
        for i in 1..concerning.len() {
            if concerning[i] == concerning[i - 1] {
                current += 1;
                max_consecutive = max_consecutive.max(current);
            } else {
                current = 1;
            }
        }

        let penalty = if max_consecutive >= 5 { 0.2 }
            else if max_consecutive >= 3 { 0.5 }
            else { 0.9 };

        // Also check uniqueness of concerning actions
        let unique: std::collections::HashSet<&&String> = concerning.iter().collect();
        let variety = (unique.len() as f64 / concerning.len() as f64).min(1.0);

        (variety * 0.4 + penalty * 0.6).clamp(0.0, 1.0)
    }

    /// Signal 3: Step efficiency.
    /// Steps that produce output vs. internal "think" steps.
    fn signal_efficiency(&self) -> f64 {
        if self.steps.is_empty() {
            return 0.5; // neutral
        }

        let productive = self.steps.iter()
            .filter(|s| s.action_kind != "think" && s.action_kind != "respond")
            .count();
        let total = self.steps.len();

        let productivity = productive as f64 / total as f64;

        // If total_output is growing, that's good
        let output_per_step = if total > 0 {
            (self.total_output as f64 / total as f64).min(10000.0) / 10000.0
        } else {
            0.0
        };

        (productivity * 0.7 + output_per_step * 0.3).clamp(0.05, 1.0)
    }

    /// Signal 4: Anomaly density.
    /// Too many dangerous actions in a short window = risky behavior.
    fn signal_anomaly(&self) -> f64 {
        if self.steps.is_empty() {
            return 1.0; // no actions = no anomalies
        }

        let dangerous = self.steps.iter().filter(|s| s.is_dangerous).count();
        let density = dangerous as f64 / self.steps.len() as f64;

        // Dense dangerous actions = low score
        // Occasional dangerous actions = OK
        if density > 0.5 { 0.2 }
        else if density > 0.3 { 0.5 }
        else if density > 0.1 { 0.8 }
        else { 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_state_is_neutral() {
        let sf = SignalFusion::new("test task".into());
        let report = sf.evaluate();
        assert!(report.confidence >= 0.3);
        assert!(report.confidence <= 0.85);
    }

    #[test]
    fn test_perfect_record_gives_high_confidence() {
        let mut sf = SignalFusion::new("test".into());
        for _ in 0..10 {
            sf.record(true, "read", 500, false);
        }
        let report = sf.evaluate();
        assert!(report.confidence > 0.8, "confidence should be high, got {:.2}", report.confidence);
    }

    #[test]
    fn test_all_failures_gives_low_confidence() {
        let mut sf = SignalFusion::new("test".into());
        for _ in 0..10 {
            sf.record(false, "execute", 0, true);
        }
        let report = sf.evaluate();
        assert!(report.confidence < 0.5, "confidence should be low, got {:.2}", report.confidence);
    }

    #[test]
    fn test_repetition_detected() {
        let mut sf = SignalFusion::new("test".into());
        // Same action 10 times in a row
        for _ in 0..10 {
            sf.record(true, "think", 100, false);
        }
        let report = sf.evaluate();
        assert!(report.repetition_score < 0.8, "repetition should be detected");
    }

    #[test]
    fn test_productive_actions_boost_efficiency() {
        let mut sf = SignalFusion::new("test".into());
        for _ in 0..5 {
            sf.record(true, "read", 2000, false);
        }
        let report = sf.evaluate();
        assert!(report.efficiency_score > 0.5);
    }
}
