/// Adaptive Gain Scheduling — closes the feedback loop between observer and controller.
///
/// The observer's signal fusion outputs drive real-time adjustments to controller
/// behavior. High confidence → less confirmation, faster pace. Low confidence →
/// more confirmation, slower pace, doubled observation.
///
/// Design: Self-Tuning Regulator with anti-windup and hysteresis.

use colored::Colorize;

#[derive(Debug, Clone, PartialEq)]
pub enum OperatingMode {
    /// Confidence high and stable — act fast, confirm little
    Aggressive,
    /// Normal operation
    Nominal,
    /// Confidence low or falling — act slow, confirm often
    Conservative,
    /// Anomaly density triggered — stop and reassess
    Safe,
}

#[derive(Debug)]
pub struct AdaptationState {
    pub mode: OperatingMode,
    /// How many steps at the current confidence level/count
    confidence_history: Vec<f64>,
    /// Consecutive counts for hysteresis (don't flip on one bad signal)
    consecutive_low: u32,
    consecutive_high: u32,
    /// Thresholds for mode transitions
    aggressive_threshold: f64,
    safe_threshold: f64,
    hysteresis: u32,
}

impl AdaptationState {
    pub fn new() -> Self {
        AdaptationState {
            mode: OperatingMode::Nominal,
            confidence_history: Vec::new(),
            consecutive_low: 0,
            consecutive_high: 0,
            aggressive_threshold: 0.75,
            safe_threshold: 0.35,
            hysteresis: 2, // need 2 consecutive signals to switch mode
        }
    }

    /// Feed a new confidence reading. Returns the updated mode.
    pub fn update(&mut self, confidence: f64) -> &OperatingMode {
        self.confidence_history.push(confidence);
        if self.confidence_history.len() > 20 {
            self.confidence_history.remove(0);
        }

        let trend = self.compute_trend();

        // Hysteresis logic — don't flip on a single bad/good reading
        if confidence < self.safe_threshold || trend < -0.1 {
            self.consecutive_low += 1;
            self.consecutive_high = 0;
        } else if confidence > self.aggressive_threshold && trend >= 0.03 {
            self.consecutive_high += 1;
            self.consecutive_low = 0;
        } else {
            // Nominal zone — reset both
            self.consecutive_low = self.consecutive_low.saturating_sub(1);
            self.consecutive_high = self.consecutive_high.saturating_sub(1);
        }

        // Mode transitions with hysteresis
        if self.consecutive_high >= self.hysteresis
            && self.mode != OperatingMode::Aggressive
        {
            self.mode = OperatingMode::Aggressive;
            self.consecutive_high = 0;
        } else if self.consecutive_low >= self.hysteresis
            && self.mode != OperatingMode::Safe
            && self.mode != OperatingMode::Conservative
        {
            self.mode = OperatingMode::Conservative;
            self.consecutive_low = 0;
        } else if self.consecutive_low >= self.hysteresis + 1
            && self.mode == OperatingMode::Conservative
        {
            // Escalating: prolonged low confidence → safe mode
            self.mode = OperatingMode::Safe;
        } else if confidence > 0.5 && trend > 0.0 {
            // Recovery: confidence rising above midpoint → back to nominal
            if self.mode == OperatingMode::Conservative || self.mode == OperatingMode::Safe {
                self.mode = OperatingMode::Nominal;
                self.consecutive_low = 0;
            }
        }

        &self.mode
    }

    /// Compute confidence trend from history (linear regression slope)
    fn compute_trend(&self) -> f64 {
        let len = self.confidence_history.len();
        if len < 3 {
            return 0.0;
        }
        // Take last 5 in chronological order (oldest → newest)
        let recent: Vec<f64> = self.confidence_history.iter()
            .rev().take(5).collect::<Vec<_>>().into_iter()
            .rev().copied().collect();

        let n = recent.len() as f64;
        if n < 3.0 { return 0.0; }

        let sum_x: f64 = (0..recent.len()).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().sum();
        let sum_xy: f64 = recent.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_xx: f64 = (0..recent.len()).map(|i| (i as f64).powi(2)).sum();

        let denominator = n * sum_xx - sum_x * sum_x;
        if denominator.abs() < 0.001 {
            return 0.0;
        }
        (n * sum_xy - sum_x * sum_y) / denominator
    }

    /// Should the observer run twice per step? (doubled observation frequency)
    pub fn should_double_observe(&self) -> bool {
        matches!(self.mode, OperatingMode::Conservative | OperatingMode::Safe)
    }

    /// Should the controller skip confirmation for safe actions?
    pub fn skip_confirm_for_safe(&self) -> bool {
        matches!(self.mode, OperatingMode::Aggressive)
    }

    /// Should ALL actions require confirmation?
    pub fn confirm_all(&self) -> bool {
        matches!(self.mode, OperatingMode::Safe)
    }

    /// Current operating mode label
    pub fn mode_label(&self) -> &str {
        match self.mode {
            OperatingMode::Aggressive => "激进",
            OperatingMode::Nominal => "标准",
            OperatingMode::Conservative => "保守",
            OperatingMode::Safe => "安全",
        }
    }

    /// Colorized mode display
    pub fn mode_display(&self) -> String {
        match self.mode {
            OperatingMode::Aggressive => "⚡ 激进".green().to_string(),
            OperatingMode::Nominal => "● 标准".to_string(),
            OperatingMode::Conservative => "⚠ 保守".yellow().to_string(),
            OperatingMode::Safe => "■ 安全模式".red().to_string(),
        }
    }

    /// Print adaptation status
    pub fn print_status(&self, confidence: f64, trend: f64) {
        println!(
            "  模式: {} | 置信度: {:.2} | 趋势: {:+.2}",
            self.mode_display(), confidence, trend
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starts_nominal() {
        let adapt = AdaptationState::new();
        assert_eq!(adapt.mode, OperatingMode::Nominal);
    }

    #[test]
    fn test_stays_nominal_with_mixed_signals() {
        let mut adapt = AdaptationState::new();
        assert_eq!(*adapt.update(0.6), OperatingMode::Nominal);
        assert_eq!(*adapt.update(0.5), OperatingMode::Nominal);
        assert_eq!(*adapt.update(0.7), OperatingMode::Nominal);
    }

    #[test]
    fn test_transitions_to_conservative_on_decline() {
        let mut adapt = AdaptationState::new();
        // Two low readings → conservative
        assert_eq!(*adapt.update(0.3), OperatingMode::Nominal);
        assert_eq!(*adapt.update(0.2), OperatingMode::Conservative);
    }

    #[test]
    fn test_transitions_to_aggressive_on_high() {
        let mut adapt = AdaptationState::new();
        adapt.update(0.75);
        adapt.update(0.80);
        adapt.update(0.85); // consecutive_high=1
        adapt.update(0.87); // consecutive_high=2 → Aggressive
        assert_eq!(*adapt.update(0.90), OperatingMode::Aggressive);
    }

    #[test]
    fn test_recovers_from_conservative() {
        let mut adapt = AdaptationState::new();
        adapt.update(0.3);
        adapt.update(0.2); // conservative
        assert_eq!(adapt.mode, OperatingMode::Conservative);
        // Recovery: confidence > 0.5 with positive trend
        assert_eq!(*adapt.update(0.6), OperatingMode::Nominal);
    }

    #[test]
    fn test_escalates_to_safe() {
        let mut adapt = AdaptationState::new();
        adapt.update(0.3);
        adapt.update(0.2); // conservative
        // Need 3 more low readings to escalate: hysteresis+1 = 3
        adapt.update(0.25);
        adapt.update(0.2);
        adapt.update(0.15);
        assert_eq!(adapt.mode, OperatingMode::Safe);
    }
}
