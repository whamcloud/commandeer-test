use anyhow::Result;
use clap::{Parser, Subcommand};
use commandeer_test::{exit_with_code, output_invocation, record_command, replay_command};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "commandeer")]
/// A CLI test binary substitute with record and replay modes
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Record {
        /// Path to save recordings.
        #[arg(long, default_value = "recordings.json")]
        file: PathBuf,
        /// Whether to truncate the file before recording.
        #[arg(long)]
        truncate: bool,
        #[arg(long)]
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Replay {
        /// Path to the recordings.
        #[arg(long, default_value = "recordings.json")]
        file: PathBuf,
        #[arg(long)]
        command: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

async fn record_mode(
    truncate: bool,
    file_path: PathBuf,
    command: String,
    args: Vec<String>,
) -> Result<()> {
    let invocation = record_command(truncate, file_path, command, args).await?;

    output_invocation(&invocation);

    exit_with_code(invocation.exit_code);
}

async fn replay_mode(file_path: PathBuf, command: String, args: Vec<String>) -> Result<()> {
    match replay_command(file_path, command.clone(), args.clone()).await? {
        Some(invocation) => {
            output_invocation(&invocation);

            exit_with_code(invocation.exit_code);
        }
        None => {
            eprintln!(
                "No recorded invocation found for: {command} {}",
                args.join(" ")
            );

            exit_with_code(1);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Record {
            file,
            command,
            args,
            truncate,
        } => {
            record_mode(truncate, file, command, args).await?;
        }
        Commands::Replay {
            file,
            command,
            args,
        } => {
            replay_mode(file, command, args).await?;
        }
    }

    Ok(())
}
