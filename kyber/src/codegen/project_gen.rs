use std::path::{Path, PathBuf};
use tera::Context;
use anyhow::{Result, anyhow};
use crate::config::types::{KyberConfig, DialogueAnswers};
use crate::codegen::template_engine::TemplateEngine;
use crate::codegen::toml_gen;

pub fn generate_project(answers: &DialogueAnswers) -> Result<PathBuf> {
    let project_dir = PathBuf::from(&answers.name);
    let engine = TemplateEngine::new()?;

    let config: KyberConfig = answers.clone().into();

    create_project_structure(&project_dir)?;
    write_generated_files(&project_dir, &config, &engine)?;
    toml_gen::write_toml(&project_dir, &config)?;

    Ok(project_dir)
}

fn create_project_structure(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::create_dir_all(dir.join("src").join("tools"))?;
    std::fs::create_dir_all(dir.join("src").join("control"))?;
    Ok(())
}

fn build_context(config: &KyberConfig) -> Context {
    let mut ctx = Context::new();
    ctx.insert("agent_name", &config.agent.name);
    ctx.insert("role", &config.agent.role);
    ctx.insert("description", &config.agent.description);
    ctx.insert("max_iterations", &config.control.controller.max_iterations);
    ctx.insert("confidence_threshold", &config.control.observer.confidence_threshold);
    ctx.insert("require_confirm", &config.control.safety.require_confirm);
    ctx.insert("circuit_breaker", &config.control.safety.circuit_breaker);
    ctx.insert("audit_log", &config.control.safety.audit_log);
    ctx.insert("strategy", &config.control.controller.strategy);
    ctx.insert("has_terminal", &config.tools.enabled.contains(&"terminal".into()));
    ctx.insert("has_filesystem", &config.tools.enabled.contains(&"filesystem".into()));
    ctx.insert("has_browser", &config.tools.enabled.contains(&"browser".into()));
    ctx.insert("has_desktop", &config.tools.enabled.contains(&"desktop".into()));
    ctx.insert("has_network", &config.tools.enabled.contains(&"network".into()));
    ctx.insert("has_git", &config.tools.enabled.contains(&"git".into()));
    ctx
}

fn write_generated_files(dir: &Path, config: &KyberConfig, engine: &TemplateEngine) -> Result<()> {
    let ctx = build_context(config);
    let arch_dir = match config.control.architecture.to_string().as_str() {
        "deep-verify" => "deep-verify",
        _ => "react",
    };

    // Template name mapping: embedded path → output path
    let main_tpl = format!("{}/src/main.rs.tera", arch_dir);
    let obs_tpl = format!("{}/src/observer.rs.tera", arch_dir);
    let ctrl_tpl = format!("{}/src/controller.rs.tera", arch_dir);

    let files: Vec<(&str, &Path)> = vec![
        ("base/Cargo.toml.tera", &Path::new("Cargo.toml")),
        ("base/src/safety.rs.tera", &Path::new("src/control/safety.rs")),
        ("base/src/tools.rs.tera", &Path::new("src/tools/mod.rs")),
        (&main_tpl, &Path::new("src/main.rs")),
        (&obs_tpl, &Path::new("src/control/observer.rs")),
        (&ctrl_tpl, &Path::new("src/control/controller.rs")),
    ];

    for (template_name, output_path) in &files {
        let rendered = engine.render(template_name, &ctx)
            .map_err(|e| anyhow!("Failed to render {}: {}", template_name, e))?;
        std::fs::write(dir.join(output_path), &rendered)?;
    }

    // Deep-verify specific: verifier module
    if config.control.architecture.to_string() == "deep-verify" {
        let verifier = engine.render("deep-verify/src/verifier.rs.tera", &ctx)?;
        std::fs::write(dir.join("src").join("control").join("verifier.rs"), &verifier)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{RiskLevel, ActionKind, Tool as ToolType, ArchitectureStyle, DialogueAnswers};

    fn test_answers() -> DialogueAnswers {
        DialogueAnswers {
            name: "test-agent".into(),
            role: "developer".into(),
            tools: vec![ToolType::Terminal, ToolType::Filesystem],
            risk_level: RiskLevel::Medium,
            confirm_actions: vec![ActionKind::Delete, ActionKind::Execute],
            architecture: ArchitectureStyle::React,
        }
    }

    #[test]
    fn test_context_builds() {
        let answers = test_answers();
        let config: KyberConfig = answers.into();
        let ctx = build_context(&config);
        assert_eq!(ctx.get("agent_name").unwrap().as_str(), Some("test-agent"));
        assert_eq!(ctx.get("has_terminal").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_create_project_structure() {
        let dir = std::env::temp_dir().join("kyber-test-structure");
        let _ = std::fs::remove_dir_all(&dir);
        create_project_structure(&dir).unwrap();
        assert!(dir.join("src").join("control").exists());
        assert!(dir.join("src").join("tools").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
