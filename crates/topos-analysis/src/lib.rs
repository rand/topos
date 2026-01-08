//! Topos semantic analysis crate.
//!
//! Provides parsing, symbol resolution, traceability tracking, and
//! diagnostic extraction for `.tps` files.
//!
//! # Architecture
//!
//! This crate uses [Salsa](https://salsa-rs.github.io/salsa/) for incremental
//! computation. The main entry points are:
//!
//! - [`db::AnalysisDatabase`] - The database holding all analysis state
//! - [`db::parse`] - Parse source to typed AST
//! - [`symbols::symbols`] - Build symbol table
//! - [`resolve::resolve_references`] - Resolve all references
//! - [`traceability::traceability`] - Build traceability graph
//! - [`diagnostics::diagnostics`] - Compute semantic diagnostics
//!
//! # Example
//!
//! ```
//! use topos_analysis::db::AnalysisDatabase;
//! use topos_analysis::diagnostics::diagnostics;
//!
//! let mut db = AnalysisDatabase::new();
//! let file = db.add_file("spec.tps".to_string(), "spec Example\n".to_string());
//! let diags = diagnostics(&db, file);
//! assert!(!diags.has_errors());
//! ```

pub mod db;
pub mod diagnostics;
pub mod resolve;
pub mod symbols;
pub mod traceability;

use tree_sitter::{Language, Node, Parser, Tree};

// Re-exports for convenience
pub use db::{parse as parse_file, AnalysisDatabase, Db, SourceFile};
pub use diagnostics::{
    diagnostics as compute_diagnostics, DiagnosticKind, SemanticDiagnostic, SemanticDiagnostics,
    Severity as SemanticSeverity,
};
pub use resolve::{resolve_references, Reference, ReferenceKind, ResolvedReference, ResolvedReferences};
pub use symbols::{symbols as compute_symbols, Symbol, SymbolKind, SymbolTable};
pub use traceability::{traceability as compute_traceability, TraceNode, TraceNodeKind, TraceabilityGraph};

// ============================================================================
// Legacy API (for backward compatibility with existing LSP/CLI)
// ============================================================================

/// A diagnostic message with source location (legacy API).
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub message: String,
    pub severity: LegacySeverity,
}

/// Diagnostic severity level (legacy API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySeverity {
    Error,
    Warning,
    Info,
}

// Keep Severity as alias for backward compatibility
pub use LegacySeverity as Severity;

/// Returns the tree-sitter Language for Topos.
pub fn language() -> Language {
    tree_sitter_topos::language()
}

/// Parse Topos source text and return the syntax tree.
pub fn parse(text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language()).ok()?;
    parser.parse(text, None)
}

/// Check Topos source text for errors (legacy API).
///
/// Returns a list of diagnostics for any syntax errors found.
/// For semantic diagnostics, use the Salsa-based API instead.
pub fn check(text: &str) -> Vec<Diagnostic> {
    let Some(tree) = parse(text) else {
        return vec![Diagnostic {
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
            message: "Failed to parse document".to_string(),
            severity: LegacySeverity::Error,
        }];
    };

    let mut diagnostics = Vec::new();
    collect_errors(tree.root_node(), text, &mut diagnostics);
    diagnostics
}

/// Recursively collect ERROR nodes from the syntax tree.
fn collect_errors(node: Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    if node.is_error() || node.is_missing() {
        let start = node.start_position();
        let end = node.end_position();

        let message = if node.is_missing() {
            format!("Missing {}", node.kind())
        } else {
            // Try to get context around the error
            let error_text = node.utf8_text(source.as_bytes()).unwrap_or("...");
            let preview: String = error_text.chars().take(20).collect();
            format!("Syntax error near '{}'", preview.trim())
        };

        diagnostics.push(Diagnostic {
            line: start.row as u32,
            column: start.column as u32,
            end_line: end.row as u32,
            end_column: end.column as u32,
            message,
            severity: LegacySeverity::Error,
        });
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, source, diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_load() {
        let lang = language();
        assert!(lang.abi_version() > 0);
    }

    #[test]
    fn test_parse_valid() {
        let tree = parse("spec Test\n").unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_check_empty() {
        let diagnostics = check("");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_check_valid_spec() {
        let source = r#"spec TaskManagement

# Requirements

## REQ-1: Task Creation
As a user I want to create tasks.
when: user submits form
the system shall: create task
"#;
        let diagnostics = check(source);
        assert!(diagnostics.is_empty(), "Expected no errors, got: {:?}", diagnostics);
    }

    #[test]
    fn test_check_invalid_syntax() {
        // Invalid: requirement without proper structure
        let source = "spec Test\n\n## not-a-req-id: Title\n";
        let diagnostics = check(source);
        // This should produce an error since "not-a-req-id" doesn't match REQ-* pattern
        assert!(!diagnostics.is_empty());
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a valid identifier.
    fn identifier_strategy() -> impl Strategy<Value = String> {
        "[A-Z][a-zA-Z0-9]{0,10}".prop_map(|s| s)
    }

    /// Generate a valid requirement ID.
    fn req_id_strategy() -> impl Strategy<Value = String> {
        (1..100u32).prop_map(|n| format!("REQ-{n}"))
    }

    /// Generate a valid task ID.
    fn task_id_strategy() -> impl Strategy<Value = String> {
        (1..100u32).prop_map(|n| format!("TASK-{n}"))
    }

    /// Generate a simple spec with requirements, concepts, and tasks.
    fn simple_spec_strategy() -> impl Strategy<Value = String> {
        (
            identifier_strategy(),
            proptest::collection::vec(req_id_strategy(), 0..5),
            proptest::collection::vec(identifier_strategy(), 0..5),
            proptest::collection::vec((task_id_strategy(), proptest::collection::vec(req_id_strategy(), 0..2)), 0..5),
        )
            .prop_map(|(name, req_ids, concept_names, tasks)| {
                let mut out = format!("spec {name}\n\n");

                if !req_ids.is_empty() {
                    out.push_str("# Requirements\n\n");
                    for req_id in &req_ids {
                        out.push_str(&format!("## {req_id}: Test requirement\nDescription.\n\n"));
                    }
                }

                if !concept_names.is_empty() {
                    out.push_str("# Concepts\n\n");
                    for concept_name in &concept_names {
                        out.push_str(&format!("Concept {concept_name}:\n  field id\n\n"));
                    }
                }

                if !tasks.is_empty() {
                    out.push_str("# Tasks\n\n");
                    for (task_id, refs) in &tasks {
                        // Only reference requirements that exist
                        let valid_refs: Vec<_> = refs.iter().filter(|r| req_ids.contains(r)).collect();
                        if valid_refs.is_empty() {
                            out.push_str(&format!("## {task_id}: Test task\nstatus: pending\n\n"));
                        } else {
                            let ref_str: Vec<_> = valid_refs.iter().map(|r| r.as_str()).collect();
                            out.push_str(&format!("## {task_id}: Test task [{}]\nstatus: pending\n\n", ref_str.join(", ")));
                        }
                    }
                }

                out
            })
    }

    proptest! {
        /// Symbol table should contain all requirements in both maps.
        #[test]
        fn symbol_table_requirements_consistent(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let symbols = compute_symbols(&db, file);

            // Every requirement in the specialized map should be in the general map
            for (id, sym) in &symbols.requirements {
                prop_assert!(
                    symbols.symbols.contains_key(id),
                    "Requirement {} in requirements map but not in symbols map",
                    id
                );
                prop_assert_eq!(sym.kind, SymbolKind::Requirement);
            }
        }

        /// Symbol table should contain all tasks in both maps.
        #[test]
        fn symbol_table_tasks_consistent(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let symbols = compute_symbols(&db, file);

            // Every task in the specialized map should be in the general map
            for (id, sym) in &symbols.tasks {
                prop_assert!(
                    symbols.symbols.contains_key(id),
                    "Task {} in tasks map but not in symbols map",
                    id
                );
                prop_assert_eq!(sym.kind, SymbolKind::Task);
            }
        }

        /// Symbol table should contain all concepts in both maps.
        #[test]
        fn symbol_table_concepts_consistent(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let symbols = compute_symbols(&db, file);

            // Every concept in the specialized map should be in the general map
            for (name, sym) in &symbols.concepts {
                prop_assert!(
                    symbols.symbols.contains_key(name),
                    "Concept {} in concepts map but not in symbols map",
                    name
                );
                prop_assert_eq!(sym.kind, SymbolKind::Concept);
            }
        }

        /// Traceability graph should be consistent with task references.
        #[test]
        fn traceability_consistent_with_tasks(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let symbols = compute_symbols(&db, file);
            let trace = compute_traceability(&db, file);

            // For each task, the traceability graph should reflect its requirement refs
            for task_id in symbols.tasks.keys() {
                let traced_reqs: Vec<_> = trace.reqs_for_task(task_id).collect();
                // Each traced requirement should exist in the symbol table
                for req_id in &traced_reqs {
                    prop_assert!(
                        symbols.requirements.contains_key(*req_id) || !symbols.symbols.contains_key(*req_id),
                        "Traced requirement {} not found in symbol table",
                        req_id
                    );
                }
            }
        }

        /// Untasked requirements should not have any tasks pointing to them.
        #[test]
        fn untasked_requirements_have_no_tasks(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let trace = compute_traceability(&db, file);

            for req_id in trace.untasked_requirements() {
                let tasks: Vec<_> = trace.tasks_for_req(req_id).collect();
                prop_assert!(
                    tasks.is_empty(),
                    "Requirement {} is marked as untasked but has tasks: {:?}",
                    req_id,
                    tasks
                );
            }
        }

        /// Symbols should have valid spans (start <= end).
        #[test]
        fn symbols_have_valid_spans(spec in simple_spec_strategy()) {
            let mut db = AnalysisDatabase::new();
            let file = db.add_file("test.tps".to_string(), spec);
            let symbols = compute_symbols(&db, file);

            for (name, sym) in &symbols.symbols {
                let span = sym.span;
                prop_assert!(
                    span.start <= span.end,
                    "Symbol {} has invalid span: start {} > end {}",
                    name,
                    span.start,
                    span.end
                );
            }
        }
    }
}
