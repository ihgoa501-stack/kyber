use crate::config::types::KyberConfig;

pub fn generate_toml_string(config: &KyberConfig) -> String {
    toml::to_string(config).expect("Config serialization should not fail")
}

pub fn write_toml(path: &std::path::Path, config: &KyberConfig) -> anyhow::Result<()> {
    let toml_str = generate_toml_string(config);
    std::fs::write(path.join("kyber.toml"), toml_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;

    fn test_config() -> KyberConfig {
        KyberConfig {
            agent: AgentConfig {
                name: "test-agent".into(),
                role: "developer".into(),
                description: "developer".into(),
            },
            control: ControlConfig {
                architecture: ArchitectureStyle::React,
                observer: ObserverConfig {
                    confidence_signals: vec!["logprobs".into()],
                    confidence_threshold: 0.7,
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
                enabled: vec!["terminal".into(), "filesystem".into()],
            },
        }
    }

    #[test]
    fn test_generates_valid_toml() {
        let config = test_config();
        let toml_str = generate_toml_string(&config);
        assert!(toml_str.contains("test-agent"));
        assert!(toml_str.contains("[agent]"));

        let parsed: KyberConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.agent.name, "test-agent");
    }
}
