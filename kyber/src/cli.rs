use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kyber", version, about = "Control-engineered Agent scaffolder")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new Kyber Agent project
    Init {
        /// Project name
        name: String,
        /// Architecture template: react | deep-verify
        #[arg(long, default_value = "react")]
        template: String,
    },
    /// Validate a Kyber project configuration
    Check {
        /// Path to project directory
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Execute a task using the Kyber Agent runtime
    Run {
        /// Task description
        task: String,
        /// Max iterations
        #[arg(long, default_value = "25")]
        max_iterations: u32,
        /// Confidence threshold (0.0-1.0)
        #[arg(long, default_value = "0.5")]
        confidence: f64,
        /// Observer provider (anthropic | openai), separate from controller
        #[arg(long)]
        observer_provider: Option<String>,
        /// Observer model override
        #[arg(long)]
        observer_model: Option<String>,
    },
}
