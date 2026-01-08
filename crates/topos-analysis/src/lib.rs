//! Topos semantic analysis crate.
//!
//! Provides parsing and diagnostic extraction for `.tps` files.

use tree_sitter::{Language, Node, Parser, Tree};

/// A diagnostic message with source location.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub message: String,
    pub severity: Severity,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

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

/// Check Topos source text for errors.
///
/// Returns a list of diagnostics for any syntax errors found.
pub fn check(text: &str) -> Vec<Diagnostic> {
    let Some(tree) = parse(text) else {
        return vec![Diagnostic {
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
            message: "Failed to parse document".to_string(),
            severity: Severity::Error,
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
            severity: Severity::Error,
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
