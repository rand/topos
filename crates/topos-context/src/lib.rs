//! AI context generation for Topos specifications.
//!
//! This crate compiles task-focused context for AI coding assistants like
//! Cursor, Windsurf, and Cline. Given a task ID, it extracts:
//! - The task definition
//! - Linked requirements
//! - Related concepts and behaviors
//! - Traceability information
//!
//! # Example
//!
//! ```
//! use topos_context::{compile_context, CompileOptions};
//! use topos_analysis::AnalysisDatabase;
//!
//! let mut db = AnalysisDatabase::new();
//! let file = db.add_file("spec.tps".to_string(), "spec Example\n\n# Requirements\n\n## REQ-1: User Login\nUsers must log in.\n\n# Tasks\n\n## TASK-1: Implement Login [REQ-1]\nstatus: pending\n".to_string());
//!
//! let context = compile_context(&db, file, "TASK-1", CompileOptions::default());
//! // Context may be None if parsing fails - that's ok for this example
//! ```

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use topos_analysis::{
    compute_symbols, compute_traceability, AnalysisDatabase, SymbolKind, SymbolTable,
    TraceabilityGraph,
};

/// Output format for compiled context.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Cursor .mdc rule file format.
    Cursor,
    /// Windsurf rules format.
    Windsurf,
    /// Cline rules format.
    Cline,
    /// Plain Markdown.
    #[default]
    Markdown,
    /// Structured JSON.
    Json,
}

/// Options for context compilation.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// Maximum depth for transitive reference collection.
    pub max_depth: Option<usize>,
    /// Maximum token budget (approximate).
    pub max_tokens: Option<usize>,
    /// Include requirement descriptions.
    pub include_descriptions: bool,
    /// Include related behaviors.
    pub include_behaviors: bool,
}

/// Compiled context for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledContext {
    /// The task ID.
    pub task_id: String,
    /// Task title/description.
    pub task_title: String,
    /// Task status.
    pub task_status: Option<String>,
    /// Linked requirements.
    pub requirements: Vec<RequirementContext>,
    /// Related concepts.
    pub concepts: Vec<ConceptContext>,
    /// Related behaviors.
    pub behaviors: Vec<BehaviorContext>,
}

/// Context for a requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementContext {
    /// Requirement ID.
    pub id: String,
    /// Requirement title.
    pub title: String,
}

/// Context for a concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptContext {
    /// Concept name.
    pub name: String,
    /// Field names.
    pub fields: Vec<String>,
}

/// Context for a behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorContext {
    /// Behavior name.
    pub name: String,
    /// Requirements it implements.
    pub implements: Vec<String>,
}

/// Compile context for a specific task.
pub fn compile_context(
    db: &AnalysisDatabase,
    file: topos_analysis::SourceFile,
    task_id: &str,
    options: CompileOptions,
) -> Option<CompiledContext> {
    let symbols = compute_symbols(db, file);
    let trace = compute_traceability(db, file);

    // Find the task
    let task_symbol = symbols.get_task(task_id)?;

    // Get linked requirements
    let req_ids: Vec<_> = trace.reqs_for_task(task_id).collect();
    let requirements: Vec<_> = req_ids
        .iter()
        .filter_map(|id| {
            symbols.get_requirement(id).map(|s| RequirementContext {
                id: s.name.clone(),
                title: s.title.clone().unwrap_or_else(|| s.name.clone()),
            })
        })
        .collect();

    // Collect related concepts
    let concepts = if options.include_behaviors {
        collect_concepts(&symbols, &trace, &req_ids)
    } else {
        vec![]
    };

    // Collect related behaviors
    let behaviors = if options.include_behaviors {
        collect_behaviors(&symbols, &trace, &req_ids)
    } else {
        vec![]
    };

    Some(CompiledContext {
        task_id: task_id.to_string(),
        task_title: task_symbol.title.clone().unwrap_or_else(|| task_symbol.name.clone()),
        task_status: task_symbol.status.clone(),
        requirements,
        concepts,
        behaviors,
    })
}

/// Collect concepts for context output.
///
/// Currently includes all concepts defined in the spec file to provide
/// complete domain vocabulary. A future enhancement could use reference
/// resolution to filter to only concepts referenced by the task's behaviors.
fn collect_concepts(
    symbols: &SymbolTable,
    _trace: &TraceabilityGraph,
    _req_ids: &[&str],
) -> Vec<ConceptContext> {
    // Include all concepts to provide complete domain vocabulary
    symbols
        .concepts.keys().map(|name| {
            let fields: Vec<_> = symbols
                .symbols
                .iter()
                .filter_map(|(field_name, sym)| {
                    if sym.kind == SymbolKind::Field && field_name.starts_with(&format!("{}.", name))
                    {
                        Some(field_name.split('.').nth(1).unwrap_or("").to_string())
                    } else {
                        None
                    }
                })
                .collect();

            ConceptContext {
                name: name.clone(),
                fields,
            }
        })
        .collect()
}

/// Collect behaviors related to requirements.
fn collect_behaviors(
    symbols: &SymbolTable,
    trace: &TraceabilityGraph,
    req_ids: &[&str],
) -> Vec<BehaviorContext> {
    let mut seen = HashSet::new();
    let mut behaviors = vec![];

    for req_id in req_ids {
        for behavior_name in trace.behaviors_for_req(req_id) {
            if seen.insert(behavior_name.to_string())
                && symbols.get_behavior(behavior_name).is_some() {
                    let implements: Vec<_> = trace
                        .reqs_for_behavior(behavior_name)
                        .map(|s| s.to_string())
                        .collect();
                    behaviors.push(BehaviorContext {
                        name: behavior_name.to_string(),
                        implements,
                    });
                }
        }
    }

    behaviors
}

/// Format compiled context for output.
pub fn format_context(context: &CompiledContext, format: OutputFormat) -> String {
    match format {
        OutputFormat::Cursor => format_cursor(context),
        OutputFormat::Windsurf => format_windsurf(context),
        OutputFormat::Cline => format_cline(context),
        OutputFormat::Markdown => format_markdown(context),
        OutputFormat::Json => format_json(context),
    }
}

/// Format as Cursor .mdc file.
fn format_cursor(context: &CompiledContext) -> String {
    let mut out = String::new();

    out.push_str("---\n");
    out.push_str(&format!("description: Task {} context\n", context.task_id));
    out.push_str("globs:\n");
    out.push_str("alwaysApply: false\n");
    out.push_str("---\n\n");

    out.push_str(&format!("# Task: {}\n\n", context.task_id));

    if !context.requirements.is_empty() {
        out.push_str("## Requirements\n\n");
        for req in &context.requirements {
            out.push_str(&format!("- **{}**: {}\n", req.id, req.title));
        }
        out.push('\n');
    }

    if !context.concepts.is_empty() {
        out.push_str("## Concepts\n\n");
        for concept in &context.concepts {
            out.push_str(&format!("### {}\n", concept.name));
            if !concept.fields.is_empty() {
                out.push_str("Fields:\n");
                for field in &concept.fields {
                    out.push_str(&format!("- {}\n", field));
                }
            }
            out.push('\n');
        }
    }

    if !context.behaviors.is_empty() {
        out.push_str("## Behaviors\n\n");
        for behavior in &context.behaviors {
            out.push_str(&format!(
                "- **{}** implements: {}\n",
                behavior.name,
                behavior.implements.join(", ")
            ));
        }
    }

    out
}

/// Format as Windsurf rules.
fn format_windsurf(context: &CompiledContext) -> String {
    // Windsurf uses similar format to Cursor
    let mut out = format_markdown(context);
    out.insert_str(0, "<!-- Windsurf Rules -->\n\n");
    out
}

/// Format as Cline rules.
fn format_cline(context: &CompiledContext) -> String {
    // Cline uses markdown with specific structure
    let mut out = String::new();

    out.push_str(&format!("# Task Context: {}\n\n", context.task_id));
    out.push_str("## Instructions\n\n");
    out.push_str("When working on this task, ensure:\n\n");

    for req in &context.requirements {
        out.push_str(&format!("- Requirement **{}** is satisfied\n", req.id));
    }
    out.push('\n');

    out.push_str(&format_markdown(context));
    out
}

/// Format as plain Markdown.
fn format_markdown(context: &CompiledContext) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", context.task_id));

    if let Some(status) = &context.task_status {
        out.push_str(&format!("**Status**: {}\n\n", status));
    }

    if !context.requirements.is_empty() {
        out.push_str("## Linked Requirements\n\n");
        for req in &context.requirements {
            out.push_str(&format!("### {}\n", req.id));
            out.push_str(&format!("{}\n\n", req.title));
        }
    }

    if !context.concepts.is_empty() {
        out.push_str("## Domain Concepts\n\n");
        for concept in &context.concepts {
            out.push_str(&format!("### {}\n", concept.name));
            if !concept.fields.is_empty() {
                out.push_str("\n| Field |\n|-------|\n");
                for field in &concept.fields {
                    out.push_str(&format!("| {} |\n", field));
                }
            }
            out.push('\n');
        }
    }

    if !context.behaviors.is_empty() {
        out.push_str("## Related Behaviors\n\n");
        for behavior in &context.behaviors {
            out.push_str(&format!("### {}\n", behavior.name));
            out.push_str(&format!("Implements: {}\n\n", behavior.implements.join(", ")));
        }
    }

    out
}

/// Format as JSON.
fn format_json(context: &CompiledContext) -> String {
    serde_json::to_string_pretty(context).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> (AnalysisDatabase, topos_analysis::SourceFile) {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-AUTH-1: User Authentication
Users must authenticate before accessing the system.

## REQ-AUTH-2: Session Management
Sessions must expire after inactivity.

# Concepts

Concept User:
  field id (`UUID`)
  field email (`Email`)
  field passwordHash (`String`)

Concept Session:
  field token (`String`)
  field userId (`UUID`)
  field expiresAt (`DateTime`)

# Tasks

## TASK-AUTH-1: Implement Login [REQ-AUTH-1]
status: pending
file: src/auth/login.rs

## TASK-AUTH-2: Add Session Timeout [REQ-AUTH-1, REQ-AUTH-2]
status: pending
"#;
        let file = db.add_file("auth.tps".to_string(), source.to_string());
        (db, file)
    }

    #[test]
    fn test_compile_context_basic() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-AUTH-1", Default::default());

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.task_id, "TASK-AUTH-1");
        assert!(!ctx.requirements.is_empty());
    }

    #[test]
    fn test_compile_context_with_behaviors() {
        let (db, file) = create_test_db();
        let options = CompileOptions {
            include_behaviors: true,
            ..Default::default()
        };
        let context = compile_context(&db, file, "TASK-AUTH-1", options);

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert!(!ctx.concepts.is_empty());
    }

    #[test]
    fn test_compile_context_missing_task() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-NONEXISTENT", Default::default());
        assert!(context.is_none());
    }

    #[test]
    fn test_format_markdown() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-AUTH-1", Default::default()).unwrap();
        let md = format_context(&context, OutputFormat::Markdown);

        assert!(md.contains("TASK-AUTH-1"));
        assert!(md.contains("REQ-AUTH-1"));
    }

    #[test]
    fn test_format_json() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-AUTH-1", Default::default()).unwrap();
        let json = format_context(&context, OutputFormat::Json);

        assert!(json.contains("\"task_id\""));
        assert!(json.contains("TASK-AUTH-1"));
    }

    #[test]
    fn test_format_cursor() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-AUTH-1", Default::default()).unwrap();
        let mdc = format_context(&context, OutputFormat::Cursor);

        assert!(mdc.contains("---"));
        assert!(mdc.contains("description:"));
    }

    #[test]
    fn test_multi_requirement_task() {
        let (db, file) = create_test_db();
        let context = compile_context(&db, file, "TASK-AUTH-2", Default::default());

        assert!(context.is_some());
        let ctx = context.unwrap();
        assert_eq!(ctx.requirements.len(), 2);
    }
}
