use std::path::PathBuf;
use crate::config::types::DialogueAnswers;
use anyhow::Result;

pub fn generate_project(_answers: &DialogueAnswers) -> Result<PathBuf> {
    Ok(PathBuf::from("."))
}
