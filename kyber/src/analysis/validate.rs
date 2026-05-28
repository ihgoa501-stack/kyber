use crate::config::types::KyberConfig;

pub fn validate_config(config: &KyberConfig) -> String {
    "kyber check — validation not yet implemented".into()
}

pub enum Severity { Error, Warning, Note }

pub struct ValidationIssue {
    pub severity: Severity,
    pub message: String,
}
