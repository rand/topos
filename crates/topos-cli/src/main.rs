use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(author, version, about = "Topos - Semantic contract language for human-AI collaboration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check a Topos file for errors
    Check {
        /// Path to the .tps file to check
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Start the Language Server
    Lsp,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match run(cli).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<bool> {
    match cli.command {
        Commands::Check { file } => {
            check_file(&file)
        }
        Commands::Lsp => {
            topos_lsp::run_server().await;
            Ok(true)
        }
    }
}

fn check_file(path: &PathBuf) -> Result<bool> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    let diagnostics = topos_analysis::check(&content);

    if diagnostics.is_empty() {
        println!("{} {}", "✓".green().bold(), path.display());
        return Ok(true);
    }

    // Print diagnostics
    let filename = path.display().to_string();
    for diag in &diagnostics {
        let severity = match diag.severity {
            topos_analysis::Severity::Error => "error".red().bold(),
            topos_analysis::Severity::Warning => "warning".yellow().bold(),
            topos_analysis::Severity::Info => "info".blue().bold(),
        };

        eprintln!(
            "{}:{}:{}: {}: {}",
            filename,
            diag.line + 1,
            diag.column + 1,
            severity,
            diag.message
        );
    }

    let error_count = diagnostics
        .iter()
        .filter(|d| matches!(d.severity, topos_analysis::Severity::Error))
        .count();

    eprintln!(
        "\n{}: {} error(s) found in {}",
        "error".red().bold(),
        error_count,
        path.display()
    );

    Ok(false)
}