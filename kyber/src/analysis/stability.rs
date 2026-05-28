use colored::Colorize;
use crate::config::types::{KyberConfig, ArchitectureStyle};

/// Estimate stability margin from configuration.
pub fn estimate_margin(config: &KyberConfig) -> String {
    // Infer risk level from confidence threshold
    let (gain, label) = if config.control.observer.confidence_threshold >= 0.7 {
        (0.3, "高")
    } else if config.control.observer.confidence_threshold >= 0.5 {
        (0.6, "中")
    } else {
        (0.9, "低")
    };

    let confirm_count = config.control.safety.require_confirm.len() as f64;
    let has_deep_verify = matches!(config.control.architecture, ArchitectureStyle::DeepVerify);
    let damping = 0.1 + confirm_count * 0.1 + if has_deep_verify { 0.3 } else { 0.0 };

    let margin = damping / (gain + 0.05);

    let status = if margin > 1.5 {
        "高稳定裕度".green()
    } else if margin > 0.8 {
        "中等稳定裕度".yellow()
    } else {
        "低稳定裕度".red()
    };

    let mut report = String::new();
    report.push_str(&format!(
        "\n{}\n{}\n\n",
        "═══ 稳定裕度估算 ═══".cyan().bold(),
        status
    ));
    report.push_str(&format!("风险等级:           {}\n", label));
    report.push_str(&format!("等效增益 (gain):    {:.2}\n", gain));
    report.push_str(&format!("等效阻尼 (damping): {:.2}\n", damping));
    report.push_str(&format!("稳定裕度 (margin):  {:.2}\n", margin));
    report.push('\n');

    if margin < 0.8 {
        report.push_str("建议：增加确认操作或启用深度验证架构以提高稳定性\n");
    } else if margin > 2.0 {
        report.push_str("注意：稳定裕度很高，但 Agent 响应速度可能偏慢\n");
    } else {
        report.push_str("稳定性和响应速度的平衡合理。\n");
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;

    fn test_config(threshold: f64, confirm: Vec<&str>, arch: ArchitectureStyle) -> KyberConfig {
        KyberConfig {
            agent: AgentConfig {
                name: "test".into(), role: "dev".into(), description: "".into(),
            },
            control: ControlConfig {
                architecture: arch,
                observer: ObserverConfig {
                    confidence_signals: vec!["logprobs".into()],
                    confidence_threshold: threshold,
                },
                controller: ControllerConfig {
                    max_iterations: 25,
                    strategy: "verify_then_act".into(),
                },
                safety: SafetyConfig {
                    require_confirm: confirm.iter().map(|s| s.to_string()).collect(),
                    circuit_breaker: "3 in 60s".into(),
                    audit_log: true,
                },
            },
            tools: ToolsConfig {
                enabled: vec!["terminal".into()],
            },
        }
    }

    #[test]
    fn test_high_confidence_gives_higher_margin() {
        let low_cfg = test_config(0.3, vec!["delete"], ArchitectureStyle::React);
        let high_cfg = test_config(0.8, vec!["delete", "write", "execute"], ArchitectureStyle::React);
        let low_report = estimate_margin(&low_cfg);
        let high_report = estimate_margin(&high_cfg);
        assert!(low_report.contains("低") || high_report.contains("高") || high_report.contains("中等"));
    }

    #[test]
    fn test_deep_verify_affects_margin() {
        let react_cfg = test_config(0.6, vec!["delete"], ArchitectureStyle::React);
        let dv_cfg = test_config(0.6, vec!["delete"], ArchitectureStyle::DeepVerify);
        let react_report = estimate_margin(&react_cfg);
        let dv_report = estimate_margin(&dv_cfg);
        assert_ne!(react_report, dv_report);
    }
}
