use anyhow::Result;
use clap::{Parser, Subcommand};

mod env;
mod docker;
mod system;
mod setup;
mod purge;

#[derive(Parser)]
#[command(name = "akk-rust")]
#[command(about = "Akkada-Stack CLI in Rust for EQEmu Server Management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Bring up the EQEmu environment
    Up {
        #[arg(short, long)]
        detached: bool,
    },
    /// Tear down the EQEmu environment
    Down,
    /// Restart containers
    Restart,
    /// Environment management
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },
    /// Show server information
    Info,
    /// Initialize the environment
    Setup,
    /// Purge legacy directories (src/, akk-stack-legacy/)
    Purge,
}

#[derive(Subcommand)]
enum EnvCommands {
    /// Merge .env.example into .env
    Transplant,
    /// Scramble secrets in .env
    Scramble {
        /// Specific key to scramble
        key: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Up { detached } => {
            docker::up(*detached).await?;
        }
        Commands::Down => {
            docker::down().await?;
        }
        Commands::Restart => {
            docker::restart().await?;
        }
        Commands::Env { action } => match action {
            EnvCommands::Transplant => {
                env::transplant()?;
            }
            EnvCommands::Scramble { key } => {
                env::scramble(key.as_deref())?;
            }
        },
        Commands::Info => {
            system::info()?;
        }
        Commands::Setup => {
            setup::execute().await?;
        }
        Commands::Purge => {
            purge::execute()?;
        }
    }

    Ok(())
}
