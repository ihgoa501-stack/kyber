use serde::{Deserialize, Serialize};

/// Architecture style for the generated Agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArchitectureStyle {
    #[serde(rename = "react")]
    React,
    #[serde(rename = "deep-verify")]
    DeepVerify,
}

impl std::fmt::Display for ArchitectureStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchitectureStyle::React => write!(f, "react"),
            ArchitectureStyle::DeepVerify => write!(f, "deep-verify"),
        }
    }
}

/// Risk level — directly maps to controller gain
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

/// Action kinds that may require human confirmation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionKind {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "write")]
    Write,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "execute")]
    Execute,
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "network")]
    Network,
}

/// Tool types the Agent can use
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Tool {
    #[serde(rename = "terminal")]
    Terminal,
    #[serde(rename = "filesystem")]
    Filesystem,
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "desktop")]
    Desktop,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "git")]
    Git,
}

impl Tool {
    pub fn is_dangerous(&self) -> bool {
        matches!(self, Tool::Desktop) // desktop can control arbitrary software
    }
}

/// Complete Kyber configuration — written to kyber.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberConfig {
    pub agent: AgentConfig,
    pub control: ControlConfig,
    pub tools: ToolsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfig {
    pub architecture: ArchitectureStyle,
    pub observer: ObserverConfig,
    pub controller: ControllerConfig,
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserverConfig {
    pub confidence_signals: Vec<String>,
    pub confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerConfig {
    pub max_iterations: u32,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub require_confirm: Vec<String>,
    pub circuit_breaker: String,
    pub audit_log: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    pub enabled: Vec<String>,
}

/// User's dialogue answers — intermediate representation before config
#[derive(Debug, Clone)]
pub struct DialogueAnswers {
    pub name: String,
    pub role: String,
    pub tools: Vec<Tool>,
    pub risk_level: RiskLevel,
    pub confirm_actions: Vec<ActionKind>,
    pub architecture: ArchitectureStyle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architecture_display() {
        assert_eq!(ArchitectureStyle::React.to_string(), "react");
        assert_eq!(ArchitectureStyle::DeepVerify.to_string(), "deep-verify");
    }

    #[test]
    fn test_tool_dangerous() {
        assert!(Tool::Desktop.is_dangerous());
        assert!(!Tool::Filesystem.is_dangerous());
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = KyberConfig {
            agent: AgentConfig {
                name: "test-agent".into(),
                role: "developer".into(),
                description: "test".into(),
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
                    require_confirm: vec!["delete".into(), "execute".into()],
                    circuit_breaker: "3 failures in 60s".into(),
                    audit_log: true,
                },
            },
            tools: ToolsConfig {
                enabled: vec!["terminal".into(), "filesystem".into()],
            },
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: KyberConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(deserialized.agent.name, "test-agent");
        assert_eq!(deserialized.control.architecture, ArchitectureStyle::React);
        assert!(!deserialized.tools.enabled.is_empty());
    }

    #[test]
    fn test_dialogue_answers_creation() {
        let answers = DialogueAnswers {
            name: "test".into(),
            role: "dev".into(),
            tools: vec![Tool::Terminal],
            risk_level: RiskLevel::Medium,
            confirm_actions: vec![ActionKind::Delete],
            architecture: ArchitectureStyle::React,
        };
        assert_eq!(answers.name, "test");
    }
}
