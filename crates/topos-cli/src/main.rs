use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use topos_analysis::AnalysisDatabase;
use topos_context::{compile_context, format_context, CompileOptions, OutputFormat};

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
    /// Start the MCP (Model Context Protocol) server
    Mcp,
    /// Show traceability report
    Trace {
        /// Path to the .tps file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Output format
        #[arg(long, short, default_value = "text")]
        format: TraceFormat,
    },
    /// Compile context for a task
    Context {
        /// Path to the .tps file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Task ID (e.g., TASK-1)
        #[arg(value_name = "TASK_ID")]
        task_id: String,
        /// Output format
        #[arg(long, short, default_value = "markdown")]
        format: ContextFormat,
        /// Include concepts and behaviors
        #[arg(long)]
        full: bool,
    },
    /// Compare two spec files and show differences
    Drift {
        /// The original/old spec file
        #[arg(value_name = "OLD")]
        old_file: PathBuf,
        /// The new spec file
        #[arg(value_name = "NEW")]
        new_file: PathBuf,
        /// Output format
        #[arg(long, short, default_value = "text")]
        format: DriftFormat,
    },
    /// Format Topos files (placeholder)
    Format {
        /// Files to format
        #[arg(value_name = "FILES")]
        files: Vec<PathBuf>,
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum TraceFormat {
    /// Human-readable text
    Text,
    /// JSON output
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum ContextFormat {
    /// Plain Markdown
    Markdown,
    /// Cursor .mdc format
    Cursor,
    /// Windsurf rules format
    Windsurf,
    /// Cline rules format
    Cline,
    /// JSON output
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum DriftFormat {
    /// Human-readable text
    Text,
    /// JSON output
    Json,
}

impl From<ContextFormat> for OutputFormat {
    fn from(f: ContextFormat) -> Self {
        match f {
            ContextFormat::Markdown => OutputFormat::Markdown,
            ContextFormat::Cursor => OutputFormat::Cursor,
            ContextFormat::Windsurf => OutputFormat::Windsurf,
            ContextFormat::Cline => OutputFormat::Cline,
            ContextFormat::Json => OutputFormat::Json,
        }
    }
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
        Commands::Check { file } => check_file(&file),
        Commands::Lsp => {
            topos_lsp::run_server().await;
            Ok(true)
        }
        Commands::Mcp => {
            topos_mcp::run_server().await?;
            Ok(true)
        }
        Commands::Trace { file, format } => trace_file(&file, format),
        Commands::Context {
            file,
            task_id,
            format,
            full,
        } => context_for_task(&file, &task_id, format, full),
        Commands::Drift {
            old_file,
            new_file,
            format,
        } => drift_files(&old_file, &new_file, format),
        Commands::Format { files, check } => format_files(&files, check),
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

fn trace_file(path: &PathBuf, format: TraceFormat) -> Result<bool> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    let mut db = AnalysisDatabase::new();
    let file = db.add_file(path.display().to_string(), content);

    let symbols = topos_analysis::compute_symbols(&db, file);
    let trace = topos_analysis::compute_traceability(&db, file);

    match format {
        TraceFormat::Text => {
            println!("{}", "Traceability Report".bold().underline());
            println!();

            // Requirements
            if !symbols.requirements.is_empty() {
                println!("{}", "Requirements:".bold());
                for id in symbols.requirements.keys() {
                    let tasks: Vec<_> = trace.tasks_for_req(id).collect();
                    let behaviors: Vec<_> = trace.behaviors_for_req(id).collect();

                    print!("  {} ", id.cyan());
                    if tasks.is_empty() && behaviors.is_empty() {
                        println!("{}", "(uncovered)".yellow());
                    } else {
                        if !tasks.is_empty() {
                            print!("→ tasks: {}", tasks.join(", ").green());
                        }
                        if !behaviors.is_empty() {
                            print!(" → behaviors: {}", behaviors.join(", ").blue());
                        }
                        println!();
                    }
                }
                println!();
            }

            // Tasks
            if !symbols.tasks.is_empty() {
                println!("{}", "Tasks:".bold());
                for id in symbols.tasks.keys() {
                    let reqs: Vec<_> = trace.reqs_for_task(id).collect();
                    print!("  {} ", id.green());
                    if reqs.is_empty() {
                        println!("{}", "(no requirements)".yellow());
                    } else {
                        println!("← {}", reqs.join(", ").cyan());
                    }
                }
                println!();
            }

            // Summary
            let untasked: Vec<_> = trace.untasked_requirements().collect();
            let uncovered: Vec<_> = trace.uncovered_requirements().collect();

            println!("{}", "Summary:".bold());
            println!(
                "  Requirements: {} total, {} untasked, {} uncovered",
                symbols.requirements.len(),
                untasked.len(),
                uncovered.len()
            );
            println!("  Tasks: {}", symbols.tasks.len());
            println!("  Concepts: {}", symbols.concepts.len());
            println!("  Behaviors: {}", symbols.behaviors.len());
        }
        TraceFormat::Json => {
            let report = serde_json::json!({
                "requirements": symbols.requirements.keys().collect::<Vec<_>>(),
                "tasks": symbols.tasks.keys().collect::<Vec<_>>(),
                "concepts": symbols.concepts.keys().collect::<Vec<_>>(),
                "behaviors": symbols.behaviors.keys().collect::<Vec<_>>(),
                "untasked_requirements": trace.untasked_requirements().collect::<Vec<_>>(),
                "uncovered_requirements": trace.uncovered_requirements().collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(true)
}

fn context_for_task(
    path: &PathBuf,
    task_id: &str,
    format: ContextFormat,
    full: bool,
) -> Result<bool> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

    let mut db = AnalysisDatabase::new();
    let file = db.add_file(path.display().to_string(), content);

    let options = CompileOptions {
        include_behaviors: full,
        include_descriptions: full,
        ..Default::default()
    };

    let context = compile_context(&db, file, task_id, options);

    match context {
        Some(ctx) => {
            let output = format_context(&ctx, format.into());
            println!("{}", output);
            Ok(true)
        }
        None => {
            eprintln!(
                "{}: Task '{}' not found in {}",
                "error".red().bold(),
                task_id,
                path.display()
            );
            Ok(false)
        }
    }
}

fn drift_files(old_path: &PathBuf, new_path: &PathBuf, format: DriftFormat) -> Result<bool> {
    let old_content = std::fs::read_to_string(old_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", old_path.display(), e))?;
    let new_content = std::fs::read_to_string(new_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", new_path.display(), e))?;

    let report = topos_diff::diff_specs(&old_content, &new_content)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if report.is_empty() {
        println!("{} No differences found", "✓".green().bold());
        return Ok(true);
    }

    match format {
        DriftFormat::Text => println!("{}", report.format_text()),
        DriftFormat::Json => println!("{}", report.format_json()),
    }

    Ok(true)
}

fn format_files(files: &[PathBuf], check: bool) -> Result<bool> {
    if files.is_empty() {
        eprintln!("{}: No files specified", "error".red().bold());
        return Ok(false);
    }

    let config = topos_syntax::FormatConfig::default();
    let mut all_ok = true;

    for path in files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;

        let file = match topos_syntax::Parser::parse(&content) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "{}: Failed to parse {}: {}",
                    "error".red().bold(),
                    path.display(),
                    e
                );
                all_ok = false;
                continue;
            }
        };

        let formatted = topos_syntax::format(&file, &config);

        if check {
            // Check mode: report if file would change
            if formatted != content {
                println!(
                    "{}: {} would be reformatted",
                    "warning".yellow().bold(),
                    path.display()
                );
                all_ok = false;
            } else {
                println!("{} {}", "✓".green().bold(), path.display());
            }
        } else {
            // Write mode: update file if changed
            if formatted != content {
                std::fs::write(path, &formatted)
                    .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", path.display(), e))?;
                println!("{} {}", "formatted".green().bold(), path.display());
            } else {
                println!("{} {} (unchanged)", "✓".green().bold(), path.display());
            }
        }
    }

    Ok(all_ok)
}
