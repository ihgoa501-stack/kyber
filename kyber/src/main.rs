use clap::Parser;
use kyber::cli::{Cli, Commands};
use anyhow::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, template } => {
            let answers = kyber::dialogue::run_dialogue(&name, template);
            let path = kyber::codegen::generate_project(&answers)?;
            println!("\n✅  已生成: {}/", path.display());
            println!("\n下一步:");
            println!("  cd {}", path.display());
            println!("  cargo build");
        }
        Commands::Check { path } => {
            let report = kyber::analysis::check_project(&path)?;
            println!("{}", report);
        }
        Commands::Run { task, max_iterations, confidence, observer_provider, observer_model } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                kyber::agent::runtime::run(task, max_iterations, confidence, observer_provider, observer_model).await
            })?;
        }
    }

    Ok(())
}
