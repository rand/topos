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
        /// Use structural comparison only (no LLM)
        #[arg(long, conflicts_with = "semantic")]
        structural: bool,
        /// Use semantic (LLM) comparison only
        #[arg(long, conflicts_with = "structural")]
        semantic: bool,
    },
    /// Format Topos files
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
    /// Extract a Topos spec from annotated Rust source files
    Extract {
        /// Paths to Rust files or directories (supports glob patterns)
        #[arg(value_name = "PATHS", required = true)]
        paths: Vec<String>,
        /// Name for the generated specification
        #[arg(long, short = 'n', default_value = "ExtractedSpec")]
        spec_name: String,
        /// Output file (prints to stdout if not specified)
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Merge with an existing spec file
        #[arg(long, short)]
        merge: Option<PathBuf>,
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
    // Load .env file if present (for API keys, etc.)
    if let Err(e) = dotenvy::dotenv() {
        // Only warn if .env exists but couldn't be read (not if it's missing)
        if e.not_found() {
            // .env doesn't exist - that's fine, use environment variables
        } else {
            eprintln!(
                "{}: Failed to load .env file: {}",
                "warning".yellow(),
                e
            );
        }
    }

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
            structural,
            semantic,
        } => drift_files(&old_file, &new_file, format, structural, semantic).await,
        Commands::Format { files, check } => format_files(&files, check),
        Commands::Gather {
            path,
            task_id,
            dry_run,
        } => gather_evidence(&path, task_id.as_deref(), dry_run),
        Commands::Extract {
            paths,
            spec_name,
            output,
            merge,
        } => extract_spec(&paths, &spec_name, output.as_ref(), merge.as_ref()),
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

async fn drift_files(
    old_path: &PathBuf,
    new_path: &PathBuf,
    format: DriftFormat,
    structural_only: bool,
    semantic_only: bool,
) -> Result<bool> {
    let old_content = std::fs::read_to_string(old_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", old_path.display(), e))?;
    let new_content = std::fs::read_to_string(new_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", new_path.display(), e))?;

    // Determine comparison strategy
    let strategy = if structural_only {
        topos_diff::ComparisonStrategy::Structural
    } else if semantic_only {
        topos_diff::ComparisonStrategy::Semantic
    } else {
        topos_diff::ComparisonStrategy::Hybrid
    };

    let options = topos_diff::SemanticDiffOptions {
        strategy,
        fallback_on_error: !semantic_only, // Don't fallback if explicitly requesting semantic
        ..Default::default()
    };

    // Perform semantic diff (falls back to structural if MCP unavailable)
    let report = topos_diff::semantic_diff(&old_content, &new_content, options)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    if !report.has_changes() {
        println!("{} No differences found", "✓".green().bold());
        if !report.semantic_available && strategy.requires_mcp() {
            println!(
                "{}: Semantic analysis unavailable (add ANTHROPIC_API_KEY to .env or use --structural)",
                "note".blue()
            );
        }
        return Ok(true);
    }

    // Show strategy info
    if !report.semantic_available && strategy.requires_mcp() {
        eprintln!(
            "{}: Semantic analysis unavailable, using structural only",
            "warning".yellow()
        );
        eprintln!("  To enable LLM-based semantic comparison:");
        eprintln!("    1. Create a .env file with: ANTHROPIC_API_KEY=sk-ant-...");
        eprintln!("    2. Or set the environment variable directly");
        eprintln!("    3. Or use --structural to skip this warning\n");
    }

    match format {
        DriftFormat::Text => println!("{}", report.format_text()),
        DriftFormat::Json => println!("{}", report.format_json()),
    }

    // Return success, but indicate if there were high-severity semantic drifts
    let drifted = report.drifted_elements(0.7);
    if !drifted.is_empty() {
        eprintln!(
            "\n{}: {} element(s) with significant semantic drift detected",
            "warning".yellow(),
            drifted.len()
        );
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

fn extract_spec(
    paths: &[String],
    spec_name: &str,
    output: Option<&PathBuf>,
    merge: Option<&PathBuf>,
) -> Result<bool> {
    use glob::glob;

    // Collect all Rust files from paths (supports glob patterns)
    let mut rust_files: Vec<PathBuf> = Vec::new();

    for pattern in paths {
        // Check if it's a glob pattern or a direct path
        if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            // Glob pattern
            for entry in glob(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid glob pattern '{}': {}", pattern, e))?
            {
                match entry {
                    Ok(path) => {
                        if path.is_file()
                            && path.extension().map(|e| e == "rs").unwrap_or(false)
                        {
                            rust_files.push(path);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to read path: {}",
                            "warning".yellow(),
                            e
                        );
                    }
                }
            }
        } else {
            let path = PathBuf::from(pattern);
            if path.is_file() {
                if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    rust_files.push(path);
                }
            } else if path.is_dir() {
                // Recursively find .rs files in directory
                collect_rust_files(&path, &mut rust_files)?;
            } else {
                eprintln!(
                    "{}: Path does not exist: {}",
                    "warning".yellow(),
                    path.display()
                );
            }
        }
    }

    if rust_files.is_empty() {
        eprintln!("{}: No Rust files found", "error".red().bold());
        return Ok(false);
    }

    println!(
        "{} {} Rust file(s)...",
        "Scanning".blue().bold(),
        rust_files.len()
    );

    // Convert PathBufs to path strings
    let file_paths: Vec<&str> = rust_files
        .iter()
        .map(|p| p.to_str().unwrap_or(""))
        .filter(|p| !p.is_empty())
        .collect();

    // Extract anchors from all files
    let collection = topos_analysis::extract_anchors_from_files(&file_paths);

    if collection.anchors.is_empty() {
        println!(
            "{}: No @topos() annotations found in {} file(s)",
            "info".blue(),
            rust_files.len()
        );
        return Ok(true);
    }

    // Report findings
    let concept_count = collection.concepts().count();
    let behavior_count = collection.behaviors().count();
    let field_count = collection.fields().count();
    let req_count = collection.requirements().count();

    println!(
        "{}: {} concept(s), {} behavior(s), {} field(s), {} requirement(s)",
        "Found".green().bold(),
        concept_count,
        behavior_count,
        field_count,
        req_count
    );

    // Generate spec
    let generated_spec = collection.generate_spec(spec_name);

    // Handle merge mode
    let final_spec = if let Some(merge_path) = merge {
        let existing = std::fs::read_to_string(merge_path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", merge_path.display(), e))?;
        merge_specs(&existing, &generated_spec)?
    } else {
        generated_spec
    };

    // Output
    if let Some(output_path) = output {
        std::fs::write(output_path, &final_spec)
            .map_err(|e| anyhow::anyhow!("Failed to write {}: {}", output_path.display(), e))?;
        println!(
            "{} {}",
            "Wrote".green().bold(),
            output_path.display()
        );
    } else {
        println!();
        println!("{}", final_spec);
    }

    Ok(true)
}

/// Recursively collect .rs files from a directory
fn collect_rust_files(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            files.push(path);
        }
    }
    Ok(())
}

/// Merge generated spec content into an existing spec
fn merge_specs(existing: &str, generated: &str) -> Result<String> {
    let mut result = existing.to_string();

    // Extract concepts section from generated
    if let Some(concepts_start) = generated.find("# Concepts") {
        let concepts_end = generated[concepts_start..]
            .find("\n# ")
            .map(|i| concepts_start + i)
            .unwrap_or(generated.len());
        let concepts_section = &generated[concepts_start..concepts_end];
        let new_concepts = extract_items_from_section(concepts_section);

        if let Some(existing_concepts) = result.find("# Concepts") {
            let existing_end = result[existing_concepts..]
                .find("\n# ")
                .map(|i| existing_concepts + i)
                .unwrap_or(result.len());
            let existing_concepts_text = result[existing_concepts..existing_end].to_string();

            // Collect concepts to add
            let concepts_to_add: Vec<_> = new_concepts
                .into_iter()
                .filter(|c| !existing_concepts_text.contains(c))
                .collect();

            // Insert at end of concepts section
            for concept in concepts_to_add.into_iter().rev() {
                result.insert_str(existing_end, &format!("\n{}", concept));
            }
        } else {
            result.push_str("\n\n");
            result.push_str(concepts_section);
        }
    }

    // Extract behaviors section from generated
    if let Some(behaviors_start) = generated.find("# Behaviors") {
        let behaviors_end = generated[behaviors_start..]
            .find("\n# ")
            .map(|i| behaviors_start + i)
            .unwrap_or(generated.len());
        let behaviors_section = &generated[behaviors_start..behaviors_end];
        let new_behaviors = extract_items_from_section(behaviors_section);

        if let Some(existing_behaviors) = result.find("# Behaviors") {
            let existing_end = result[existing_behaviors..]
                .find("\n# ")
                .map(|i| existing_behaviors + i)
                .unwrap_or(result.len());
            let existing_behaviors_text = result[existing_behaviors..existing_end].to_string();

            let behaviors_to_add: Vec<_> = new_behaviors
                .into_iter()
                .filter(|b| !existing_behaviors_text.contains(b))
                .collect();

            for behavior in behaviors_to_add.into_iter().rev() {
                result.insert_str(existing_end, &format!("\n{}", behavior));
            }
        } else {
            result.push_str("\n\n");
            result.push_str(behaviors_section);
        }
    }

    Ok(result)
}

/// Extract individual items (Concept X:, Behavior Y:) from a section
fn extract_items_from_section(section: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current_item = String::new();
    let mut in_item = false;

    for line in section.lines() {
        if line.starts_with("Concept ") || line.starts_with("Behavior ") {
            if in_item && !current_item.is_empty() {
                items.push(current_item.trim_end().to_string());
            }
            current_item = line.to_string();
            in_item = true;
        } else if in_item {
            current_item.push('\n');
            current_item.push_str(line);
        }
    }

    if in_item && !current_item.is_empty() {
        items.push(current_item.trim_end().to_string());
    }

    items
}
