//! Semantic diagnostics for Topos specifications.
//!
//! This module generates diagnostics beyond syntax errors, including:
//! - Unresolved references
//! - Missing traceability (requirements without tasks)
//! - Duplicate definitions

use std::sync::Arc;

use topos_syntax::Span;

use crate::anchors::{AnchorReferenceKind, AnchorValidation};
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
    /// Anchor references non-existent spec element.
    InvalidAnchor,
    /// Spec element has no code anchor implementation.
    OrphanSpecElement,
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

// ============================================================================
// Anchor Diagnostics
// ============================================================================

/// Generate diagnostics from anchor validation results.
///
/// This function creates diagnostics for:
/// - Invalid anchors (references to non-existent spec elements)
/// - Orphan spec elements (spec elements with no code implementation)
///
/// Since anchors come from Rust files, the diagnostics include file path and
/// line number information rather than Topos span locations.
pub fn anchor_diagnostics(validation: &AnchorValidation) -> SemanticDiagnostics {
    let mut diags = SemanticDiagnostics::default();

    // Report invalid anchors as errors
    for invalid in &validation.invalid {
        let kind_str = match invalid.reference_kind {
            AnchorReferenceKind::Requirement => "requirement",
            AnchorReferenceKind::Concept => "concept",
            AnchorReferenceKind::Behavior => "behavior",
        };

        let suggestion_str = if !invalid.suggestions.is_empty() {
            format!(". Did you mean: {}?", invalid.suggestions.join(", "))
        } else {
            String::new()
        };

        diags.diagnostics.push(SemanticDiagnostic {
            kind: DiagnosticKind::InvalidAnchor,
            message: format!(
                "Anchor references undefined {} '{}' at {}:{}{}",
                kind_str,
                invalid.unresolved_reference,
                invalid.anchor.file_path,
                invalid.anchor.line + 1, // 1-indexed for display
                suggestion_str
            ),
            span: Span::dummy(), // Anchors are in Rust files, not Topos files
            severity: Severity::Error,
        });
    }

    // Report orphan spec elements as warnings
    for orphan in &validation.orphan_spec_elements {
        let kind_str = match orphan.kind {
            AnchorReferenceKind::Requirement => "Requirement",
            AnchorReferenceKind::Concept => "Concept",
            AnchorReferenceKind::Behavior => "Behavior",
        };

        diags.diagnostics.push(SemanticDiagnostic {
            kind: DiagnosticKind::OrphanSpecElement,
            message: format!(
                "{} '{}' has no @topos anchor in code",
                kind_str, orphan.name
            ),
            span: Span::dummy(),
            severity: Severity::Warning,
        });
    }

    diags
}

/// Anchor diagnostic with source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorDiagnostic {
    /// The diagnostic kind.
    pub kind: DiagnosticKind,
    /// The message.
    pub message: String,
    /// Source file path.
    pub file_path: String,
    /// Line number (0-indexed).
    pub line: usize,
    /// Severity level.
    pub severity: Severity,
}

/// Generate detailed anchor diagnostics with file locations.
pub fn detailed_anchor_diagnostics(validation: &AnchorValidation) -> Vec<AnchorDiagnostic> {
    let mut diags = Vec::new();

    // Report invalid anchors as errors
    for invalid in &validation.invalid {
        let kind_str = match invalid.reference_kind {
            AnchorReferenceKind::Requirement => "requirement",
            AnchorReferenceKind::Concept => "concept",
            AnchorReferenceKind::Behavior => "behavior",
        };

        let suggestion_str = if !invalid.suggestions.is_empty() {
            format!(". Did you mean: {}?", invalid.suggestions.join(", "))
        } else {
            String::new()
        };

        diags.push(AnchorDiagnostic {
            kind: DiagnosticKind::InvalidAnchor,
            message: format!(
                "Anchor references undefined {} '{}'{}",
                kind_str, invalid.unresolved_reference, suggestion_str
            ),
            file_path: invalid.anchor.file_path.clone(),
            line: invalid.anchor.line,
            severity: Severity::Error,
        });
    }

    // Report orphan spec elements as warnings (no specific file location)
    for orphan in &validation.orphan_spec_elements {
        let kind_str = match orphan.kind {
            AnchorReferenceKind::Requirement => "Requirement",
            AnchorReferenceKind::Concept => "Concept",
            AnchorReferenceKind::Behavior => "Behavior",
        };

        diags.push(AnchorDiagnostic {
            kind: DiagnosticKind::OrphanSpecElement,
            message: format!("{} '{}' has no @topos anchor in code", kind_str, orphan.name),
            file_path: String::new(), // Spec-level warning
            line: 0,
            severity: Severity::Warning,
        });
    }

    diags
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

    // ========================================================================
    // Anchor diagnostics tests
    // ========================================================================

    #[test]
    fn test_anchor_diagnostics_invalid_reference() {
        use crate::anchors::{extract_anchors, validate_anchors};
        use crate::symbols::SymbolTable;

        // Create anchors referencing non-existent spec elements
        let rust_source = r#"
// @topos(req="REQ-NONEXISTENT", concept="MissingConcept")
pub struct Order {}
"#;
        let anchors = extract_anchors(rust_source, "test.rs");

        // Empty symbol table - no spec elements defined
        let symbols = SymbolTable::new();
        let validation = validate_anchors(&anchors, &symbols);

        let diags = anchor_diagnostics(&validation);
        assert!(diags.has_errors());

        let errors: Vec<_> = diags.errors().collect();
        assert_eq!(errors.len(), 2); // REQ-NONEXISTENT and MissingConcept

        assert!(errors.iter().any(|e| e.kind == DiagnosticKind::InvalidAnchor));
        assert!(errors.iter().any(|e| e.message.contains("REQ-NONEXISTENT")));
        assert!(errors.iter().any(|e| e.message.contains("MissingConcept")));
    }

    #[test]
    fn test_anchor_diagnostics_orphan_elements() {
        use crate::anchors::{extract_anchors, validate_anchors};
        use crate::symbols::{Symbol, SymbolKind, SymbolTable};
        use topos_syntax::Span;

        // No anchors
        let anchors = extract_anchors("pub fn foo() {}", "test.rs");

        // Symbol table with spec elements
        let mut symbols = SymbolTable::new();
        symbols.add(Symbol {
            name: "REQ-ORPHAN".to_string(),
            kind: SymbolKind::Requirement,
            title: Some("Orphan Requirement".to_string()),
            status: None,
            file: None,
            tests: None,
            span: Span::dummy(),
        });

        let validation = validate_anchors(&anchors, &symbols);
        let diags = anchor_diagnostics(&validation);

        let warnings: Vec<_> = diags.warnings().collect();
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.kind == DiagnosticKind::OrphanSpecElement));
        assert!(warnings.iter().any(|w| w.message.contains("REQ-ORPHAN")));
    }

    #[test]
    fn test_detailed_anchor_diagnostics() {
        use crate::anchors::{extract_anchors, validate_anchors};
        use crate::symbols::SymbolTable;

        let rust_source = r#"
// @topos(concept="NonExistent")
pub struct Test {}
"#;
        let anchors = extract_anchors(rust_source, "src/models.rs");
        let symbols = SymbolTable::new();
        let validation = validate_anchors(&anchors, &symbols);

        let diags = detailed_anchor_diagnostics(&validation);
        assert!(!diags.is_empty());

        let error = &diags[0];
        assert_eq!(error.kind, DiagnosticKind::InvalidAnchor);
        assert_eq!(error.file_path, "src/models.rs");
        assert_eq!(error.line, 1); // 0-indexed
        assert!(error.message.contains("NonExistent"));
    }
}
