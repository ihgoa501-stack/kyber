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
}
