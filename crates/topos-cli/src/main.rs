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
    /// Gather evidence for tasks from git history
    Gather {
        /// Path to spec file or directory containing .tps files
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
        /// Specific task ID to gather evidence for
        #[arg(value_name = "TASK_ID")]
        task_id: Option<String>,
        /// Preview changes without modifying files
        #[arg(long)]
        dry_run: bool,
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
        Commands::Gather {
            path,
            task_id,
            dry_run,
        } => gather_evidence(&path, task_id.as_deref(), dry_run),
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

fn gather_evidence(path: &PathBuf, task_id: Option<&str>, dry_run: bool) -> Result<bool> {
    use git2::Repository;

    // Find the git repository
    let repo = Repository::discover(path)
        .map_err(|e| anyhow::anyhow!("Not a git repository: {}", e))?;

    // Collect spec files to process
    let spec_files: Vec<PathBuf> = if path.is_file() {
        vec![path.clone()]
    } else {
        walkdir(path)?
            .into_iter()
            .filter(|p| {
                p.extension()
                    .map(|e| e == "tps" || e == "topos")
                    .unwrap_or(false)
            })
            .collect()
    };

    if spec_files.is_empty() {
        println!("{}: No .tps or .topos files found", "warning".yellow());
        return Ok(true);
    }

    let mut total_updates = 0;

    for spec_path in &spec_files {
        let content = std::fs::read_to_string(spec_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", spec_path.display(), e))?;

        let mut db = AnalysisDatabase::new();
        let file = db.add_file(spec_path.display().to_string(), content.clone());
        let symbols = topos_analysis::compute_symbols(&db, file);

        // Process tasks
        for (id, task) in &symbols.tasks {
            // Skip if specific task requested and this isn't it
            if let Some(target_id) = task_id {
                if id != target_id {
                    continue;
                }
            }

            // Get file paths from task
            let file_paths = extract_file_paths(task);
            if file_paths.is_empty() {
                continue;
            }

            // Gather evidence for each file
            let mut evidence = GatheredEvidence::default();

            for file_path in &file_paths {
                if let Some(commit_info) = get_latest_commit(&repo, file_path) {
                    evidence.add_commit(commit_info);
                }
            }

            if evidence.is_empty() {
                continue;
            }

            total_updates += 1;

            if dry_run {
                println!(
                    "{}: {} in {}",
                    "would update".blue().bold(),
                    id.cyan(),
                    spec_path.display()
                );
                println!("  evidence:");
                if let Some(commit) = &evidence.latest_commit {
                    println!("    commit: {}", commit);
                }
                if !evidence.files_updated.is_empty() {
                    println!(
                        "    files: {}",
                        evidence.files_updated.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                    );
                }
                println!();
            } else {
                // For now, just report what we would do
                // Full implementation would modify the spec file
                println!(
                    "{}: {} in {}",
                    "gathered".green().bold(),
                    id.cyan(),
                    spec_path.display()
                );
                if let Some(commit) = &evidence.latest_commit {
                    println!("  commit: {}", commit);
                }
            }
        }
    }

    if total_updates == 0 {
        if task_id.is_some() {
            println!(
                "{}: Task '{}' not found or has no file references",
                "warning".yellow(),
                task_id.unwrap()
            );
        } else {
            println!("{}: No tasks with file references found", "info".blue());
        }
    } else if dry_run {
        println!(
            "\n{}: {} task(s) would be updated (use without --dry-run to apply)",
            "summary".bold(),
            total_updates
        );
    } else {
        println!(
            "\n{}: {} task(s) updated",
            "summary".bold(),
            total_updates
        );
    }

    Ok(true)
}

/// Simple directory walker (no walkdir crate dependency)
fn walkdir(path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walkdir_recursive(path, &mut files)?;
    Ok(files)
}

fn walkdir_recursive(path: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        files.push(path.clone());
        return Ok(());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walkdir_recursive(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// Extract file paths from a task symbol
fn extract_file_paths(task: &topos_analysis::Symbol) -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(file) = &task.file {
        paths.push(file.clone());
    }
    if let Some(tests) = &task.tests {
        paths.push(tests.clone());
    }
    paths
}

#[derive(Default)]
struct GatheredEvidence {
    latest_commit: Option<String>,
    files_updated: Vec<String>,
}

impl GatheredEvidence {
    fn add_commit(&mut self, info: CommitInfo) {
        // Keep the most recent commit
        if self.latest_commit.is_none() {
            self.latest_commit = Some(info.short_id);
        }
        self.files_updated.push(info.file_path);
    }

    fn is_empty(&self) -> bool {
        self.latest_commit.is_none()
    }
}

struct CommitInfo {
    short_id: String,
    file_path: String,
}

fn get_latest_commit(repo: &git2::Repository, file_path: &str) -> Option<CommitInfo> {
    // Get HEAD
    let head = repo.head().ok()?;
    let head_commit = head.peel_to_commit().ok()?;

    // Walk the commit history looking for changes to this file
    let mut revwalk = repo.revwalk().ok()?;
    revwalk.push(head_commit.id()).ok()?;
    revwalk.set_sorting(git2::Sort::TIME).ok()?;

    for oid in revwalk.flatten().take(100) {
        let commit = repo.find_commit(oid).ok()?;

        // Check if this commit touches the file
        if commit_touches_file(repo, &commit, file_path) {
            let short_id = commit.id().to_string()[..7].to_string();
            return Some(CommitInfo {
                short_id,
                file_path: file_path.to_string(),
            });
        }
    }

    None
}

fn commit_touches_file(_repo: &git2::Repository, commit: &git2::Commit, file_path: &str) -> bool {
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return false,
    };

    // Check if file exists in this commit's tree
    if tree.get_path(std::path::Path::new(file_path)).is_ok() {
        // Check if it changed from parent
        if commit.parent_count() == 0 {
            return true; // Initial commit
        }

        if let Ok(parent) = commit.parent(0) {
            if let Ok(parent_tree) = parent.tree() {
                let old_entry = parent_tree.get_path(std::path::Path::new(file_path));
                let new_entry = tree.get_path(std::path::Path::new(file_path));

                match (old_entry, new_entry) {
                    (Ok(old), Ok(new)) => old.id() != new.id(),
                    (Err(_), Ok(_)) => true, // File added
                    _ => false,
                }
            } else {
                true
            }
        } else {
            true
        }
    } else {
        false
    }
}
