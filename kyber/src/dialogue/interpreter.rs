use crate::config::types::*;

impl From<DialogueAnswers> for KyberConfig {
    fn from(answers: DialogueAnswers) -> Self {
        let (confidence_threshold, max_iterations, circuit_breaker) =
            match answers.risk_level {
                RiskLevel::Low => (0.4, 50, "5 failures in 60s"),
                RiskLevel::Medium => (0.6, 25, "3 failures in 60s"),
                RiskLevel::High => (0.8, 15, "2 failures in 60s"),
            };

        let max_iterations = if matches!(answers.architecture, ArchitectureStyle::DeepVerify) {
            max_iterations * 2
        } else {
            max_iterations
        };

        let safety_strategy = match answers.risk_level {
            RiskLevel::High => "deep_verify",
            _ => "verify_then_act",
        };

        KyberConfig {
            agent: AgentConfig {
                name: answers.name.clone(),
                role: answers.role.clone(),
                description: answers.role,
            },
            control: ControlConfig {
                architecture: answers.architecture,
                observer: ObserverConfig {
                    confidence_signals: vec!["logprobs".into(), "tool_success_rate".into()],
                    confidence_threshold,
                },
                controller: ControllerConfig {
                    max_iterations,
                    strategy: safety_strategy.into(),
                },
                safety: SafetyConfig {
                    require_confirm: answers.confirm_actions.iter()
                        .map(|a| format!("{:?}", a).to_lowercase())
                        .collect(),
                    circuit_breaker: circuit_breaker.into(),
                    audit_log: true,
                },
            },
            tools: ToolsConfig {
                enabled: answers.tools.iter()
                    .map(|t| format!("{:?}", t).to_lowercase())
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_answers(risk: RiskLevel) -> DialogueAnswers {
        DialogueAnswers {
            name: "test".into(),
            role: "developer".into(),
            tools: vec![Tool::Terminal, Tool::Filesystem],
            risk_level: risk,
            confirm_actions: vec![ActionKind::Delete],
            architecture: ArchitectureStyle::React,
        }
    }

    #[test]
    fn test_low_risk_maps_to_loose_params() {
        let config: KyberConfig = make_answers(RiskLevel::Low).into();
        assert_eq!(config.control.observer.confidence_threshold, 0.4);
        assert_eq!(config.control.controller.max_iterations, 50);
    }

    #[test]
    fn test_high_risk_maps_to_strict_params() {
        let config: KyberConfig = make_answers(RiskLevel::High).into();
        assert_eq!(config.control.observer.confidence_threshold, 0.8);
        assert_eq!(config.control.controller.max_iterations, 15);
    }

    #[test]
    fn test_confirm_actions_inherited() {
        let config: KyberConfig = make_answers(RiskLevel::Medium).into();
        assert!(config.control.safety.require_confirm.contains(&"delete".to_string()));
    }

    #[test]
    fn test_deep_verify_architecture_doubles_iterations() {
        let mut answers = make_answers(RiskLevel::Medium);
        answers.architecture = ArchitectureStyle::DeepVerify;
        let config: KyberConfig = answers.into();
        assert_eq!(config.control.controller.max_iterations, 50);
    }
}
