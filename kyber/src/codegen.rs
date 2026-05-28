use std::path::PathBuf;
use crate::dialogue::Answers;
use anyhow::Result;

pub fn generate_project(_answers: &Answers) -> Result<PathBuf> {
    Ok(PathBuf::from("."))
}
