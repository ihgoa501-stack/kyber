use colored::Colorize;
use crate::config::types::KyberConfig;

pub fn validate_config(config: &KyberConfig) -> String {
    let mut issues = vec![];

    // 1. Confidence threshold bounds
    if config.control.observer.confidence_threshold < 0.1 {
        issues.push(format!("⚠ 置信度门限极低 ({:.1})，Agent 可能在不确定时也行动",
            config.control.observer.confidence_threshold));
    }
    if config.control.observer.confidence_threshold > 0.95 {
        issues.push(format!("ℹ 置信度门限很高 ({:.1})，Agent 会频繁请示用户",
            config.control.observer.confidence_threshold));
    }

    // 2. Iteration bounds
    if config.control.controller.max_iterations == 0 {
        issues.push("✗ max_iterations 不能为 0".into());
    }
    if config.control.controller.max_iterations > 100 {
        issues.push(format!("⚠ max_iterations {} 较大，注意死循环风险",
            config.control.controller.max_iterations));
    }
    if config.control.controller.max_iterations < 3 {
        issues.push("⚠ max_iterations 过小 (< 3)，Agent 可能无法完成任务".into());
    }

    // 3. Tools and confirm match
    let has_dangerous_tool = config.tools.enabled.iter().any(|t| {
        matches!(t.as_str(), "delete" | "execute" | "desktop")
    });
    let no_confirm = config.control.safety.require_confirm.is_empty();
    if has_dangerous_tool && no_confirm {
        issues.push("✗ 启用了危险工具但没有设置确认要求，这是不安全的".into());
    }

    // 4. Audit log
    if !config.control.safety.audit_log {
        issues.push("⚠ 审计日志已关闭，将无法追溯 Agent 行为".into());
    }

    format_report(&issues, config)
}

fn format_report(issues: &[String], _config: &KyberConfig) -> String {
    let mut report = String::new();
    report.push_str(&format!("\n{}\n", "═══ Kyber 安全检查报告 ═══".cyan().bold()));

    let errors = issues.iter().filter(|i| i.starts_with("✗")).count();
    let warnings = issues.iter().filter(|i| i.starts_with("⚠")).count();

    if issues.is_empty() {
        report.push_str(&format!("{}\n", "✓ 所有检查通过，配置合理".green()));
    } else {
        for issue in issues {
            report.push_str(&format!("{}\n", issue));
        }
    }

    report.push('\n');
    report.push_str(&format!("{} 个错误，{} 个警告\n", errors, warnings));

    let grade = match (errors, warnings) {
        (0, 0) => "安全等级：A（优秀）",
        (0, 1..=2) => "安全等级：B（良好）",
        (0, _) => "安全等级：C（一般）",
        _ => "安全等级：F（不安全）",
    };
    report.push_str(&format!("{}\n", grade));

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;

    fn safe_config() -> KyberConfig {
        KyberConfig {
            agent: AgentConfig {
                name: "test".into(), role: "dev".into(), description: "".into(),
            },
            control: ControlConfig {
                architecture: ArchitectureStyle::React,
                observer: ObserverConfig {
                    confidence_signals: vec!["logprobs".into()],
                    confidence_threshold: 0.6,
                },
                controller: ControllerConfig {
                    max_iterations: 25,
                    strategy: "verify_then_act".into(),
                },
                safety: SafetyConfig {
                    require_confirm: vec!["delete".into()],
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
    fn test_safe_config_passes() {
        let report = validate_config(&safe_config());
        assert!(report.contains("A（优秀）") || report.contains("B（良好）"));
    }

    #[test]
    fn test_no_confirm_with_dangerous_tools_errors() {
        let mut config = safe_config();
        config.tools.enabled = vec!["delete".into(), "execute".into()];
        config.control.safety.require_confirm = vec![];
        let report = validate_config(&config);
        assert!(report.contains("不安全"));
    }

    #[test]
    fn test_zero_iterations_errors() {
        let mut config = safe_config();
        config.control.controller.max_iterations = 0;
        let report = validate_config(&config);
        assert!(report.contains("不能为 0"));
    }
}
