//! Semantic diagnostics for Topos specifications.
//!
//! This module generates diagnostics beyond syntax errors, including:
//! - Unresolved references
//! - Missing traceability (requirements without tasks)
//! - Duplicate definitions

use std::sync::Arc;

use topos_syntax::Span;

use crate::db::{self, Db};
use crate::resolve::{resolve_references, ReferenceKind};
use crate::symbols::symbols;
use crate::traceability::traceability;

/// A semantic diagnostic message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDiagnostic {
    /// The diagnostic kind.
    pub kind: DiagnosticKind,
    /// The message.
    pub message: String,
    /// Source location.
    pub span: Span,
    /// Severity level.
    pub severity: Severity,
}

/// Kind of semantic diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// Reference to undefined symbol.
    UnresolvedReference,
    /// Requirement has no implementing behaviors.
    UncoveredRequirement,
    /// Requirement has no referencing tasks.
    UntaskedRequirement,
    /// Duplicate symbol definition.
    DuplicateDefinition,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error that should be fixed.
    Error,
    /// Warning that may indicate a problem.
    Warning,
    /// Informational hint.
    Hint,
}

/// Collection of semantic diagnostics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticDiagnostics {
    pub diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticDiagnostics {
    /// Get all errors.
    pub fn errors(&self) -> impl Iterator<Item = &SemanticDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    /// Get all warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &SemanticDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

/// Compute all semantic diagnostics for a file.
#[salsa::tracked]
pub fn diagnostics(db: &dyn Db, file: db::SourceFile) -> Arc<SemanticDiagnostics> {
    let mut diags = SemanticDiagnostics::default();

    // Check for unresolved references
    check_unresolved_references(db, file, &mut diags);

    // Check traceability coverage
    check_traceability(db, file, &mut diags);

    Arc::new(diags)
}

/// Check for unresolved references.
fn check_unresolved_references(db: &dyn Db, file: db::SourceFile, diags: &mut SemanticDiagnostics) {
    let resolved = resolve_references(db, file);

    for unresolved in resolved.unresolved() {
        let kind_str = match unresolved.reference.kind {
            ReferenceKind::Type => "type",
            ReferenceKind::Requirement => "requirement",
            ReferenceKind::Task => "task",
            ReferenceKind::Concept => "concept",
        };

        // Only report errors for non-primitive types
        if !is_primitive_type(&unresolved.reference.name) {
            diags.diagnostics.push(SemanticDiagnostic {
                kind: DiagnosticKind::UnresolvedReference,
                message: format!(
                    "Unresolved {}: '{}'",
                    kind_str, unresolved.reference.name
                ),
                span: unresolved.reference.span,
                severity: Severity::Error,
            });
        }
    }
}

/// Check if a type name is a primitive/builtin type.
fn is_primitive_type(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "Int"
            | "Integer"
            | "Float"
            | "Bool"
            | "Boolean"
            | "Date"
            | "DateTime"
            | "Time"
            | "Duration"
            | "UUID"
            | "Identifier"
            | "Email"
            | "URL"
            | "Money"
            | "Decimal"
    )
}

/// Check traceability coverage.
fn check_traceability(db: &dyn Db, file: db::SourceFile, diags: &mut SemanticDiagnostics) {
    let graph = traceability(db, file);
    let symbol_table = symbols(db, file);

    // Warn about requirements without tasks
    for req_id in graph.untasked_requirements() {
        if let Some(symbol) = symbol_table.get_requirement(req_id) {
            diags.diagnostics.push(SemanticDiagnostic {
                kind: DiagnosticKind::UntaskedRequirement,
                message: format!("Requirement '{}' has no implementing tasks", req_id),
                span: symbol.span,
                severity: Severity::Warning,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_no_diagnostics_for_valid_spec() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: First Requirement
Description.

# Tasks

## TASK-1: Implement First [REQ-1]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let diags = diagnostics(&db, file);

        // Should have no errors (primitive types are ignored)
        assert!(!diags.has_errors(), "Expected no errors: {:?}", diags.diagnostics);
    }

    #[test]
    fn test_unresolved_requirement_reference() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Tasks

## TASK-1: Implement Missing [REQ-MISSING]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let diags = diagnostics(&db, file);

        assert!(diags.has_errors(), "Expected unresolved reference error");
        assert!(diags
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::UnresolvedReference));
    }

    #[test]
    fn test_untasked_requirement_warning() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: Lonely Requirement
No tasks reference this.
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let diags = diagnostics(&db, file);

        let warnings: Vec<_> = diags.warnings().collect();
        assert!(!warnings.is_empty(), "Expected untasked requirement warning");
        assert!(warnings
            .iter()
            .any(|d| d.kind == DiagnosticKind::UntaskedRequirement));
    }

    #[test]
    fn test_primitive_types_not_errors() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept User:
  field name (`String`)
  field age (`Int`)
  field active (`Bool`)
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let diags = diagnostics(&db, file);

        // Should have no unresolved reference errors for primitives
        let errors: Vec<_> = diags.errors().collect();
        assert!(
            errors.is_empty(),
            "Primitive types should not cause errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_unresolved_concept_reference() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept Post:
  field author (`UnknownType`)
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let diags = diagnostics(&db, file);

        assert!(diags.has_errors(), "Expected unresolved type error");
        let error = diags.errors().next().unwrap();
        assert!(error.message.contains("UnknownType"));
    }
}
