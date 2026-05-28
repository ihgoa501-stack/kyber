use std::path::Path;
use anyhow::{bail, Result};

pub mod validate;
pub mod stability;

pub fn check_project(path: &Path) -> Result<String> {
    let config_path = path.join("kyber.toml");
    if !config_path.exists() {
        bail!("No kyber.toml found in {}", path.display());
    }
    let contents = std::fs::read_to_string(&config_path)?;
    let config: crate::config::types::KyberConfig = toml::from_str(&contents)?;
    let validations = validate::validate_config(&config);
    let stability_report = stability::estimate_margin(&config);
    Ok(format!("{}\n{}", validations, stability_report))
}
