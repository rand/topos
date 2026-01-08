//! Reference resolution for Topos specifications.
//!
//! This module resolves references (backtick references, REQ-* IDs, etc.)
//! to their definitions in the symbol table.

use std::sync::Arc;

use topos_syntax::{
    Behavior, Field, SectionContent, SourceFile, Span, Task, TypeExpr,
};

use crate::db::{self, Db};
use crate::symbols::{symbols, Symbol, SymbolTable};

/// A reference to a symbol in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    /// The referenced name.
    pub name: String,
    /// The kind of reference.
    pub kind: ReferenceKind,
    /// Source location of the reference.
    pub span: Span,
}

/// Kinds of references in Topos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceKind {
    /// A type reference (backtick notation): `TypeName`.
    Type,
    /// A requirement reference: REQ-*.
    Requirement,
    /// A task reference: TASK-*.
    Task,
    /// A concept reference in prose or implements clause.
    Concept,
}

/// Result of resolving a reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReference {
    /// The original reference.
    pub reference: Reference,
    /// The resolved symbol, if found.
    pub symbol: Option<Symbol>,
}

/// All references in a file with their resolution status.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedReferences {
    /// All references found in the file.
    pub references: Vec<ResolvedReference>,
}

impl ResolvedReferences {
    /// Get all unresolved references.
    pub fn unresolved(&self) -> impl Iterator<Item = &ResolvedReference> {
        self.references.iter().filter(|r| r.symbol.is_none())
    }

    /// Get all resolved references.
    pub fn resolved(&self) -> impl Iterator<Item = &ResolvedReference> {
        self.references.iter().filter(|r| r.symbol.is_some())
    }
}

/// Collect and resolve all references in a file.
#[salsa::tracked]
pub fn resolve_references(db: &dyn Db, file: db::SourceFile) -> Arc<ResolvedReferences> {
    let ast = db::parse(db, file);
    let symbol_table = symbols(db, file);

    let mut references = Vec::new();
    collect_references(&ast, &symbol_table, &mut references);

    Arc::new(ResolvedReferences { references })
}

/// Collect all references from an AST and resolve them.
fn collect_references(
    ast: &SourceFile,
    symbols: &SymbolTable,
    references: &mut Vec<ResolvedReference>,
) {
    for section in &ast.sections {
        for content in &section.contents {
            match content {
                SectionContent::Concept(concept) => {
                    // Collect type references from fields
                    for field in &concept.fields {
                        collect_field_references(field, symbols, references);
                    }
                }
                SectionContent::Behavior(behavior) => {
                    collect_behavior_references(behavior, symbols, references);
                }
                SectionContent::Task(task) => {
                    collect_task_references(task, symbols, references);
                }
                _ => {}
            }
        }
    }
}

/// Collect references from a field definition.
fn collect_field_references(
    field: &Field,
    symbols: &SymbolTable,
    references: &mut Vec<ResolvedReference>,
) {
    if let Some(ty) = &field.ty {
        collect_type_references(ty, symbols, references);
    }
}

/// Collect references from a type expression.
fn collect_type_references(
    ty: &TypeExpr,
    symbols: &SymbolTable,
    references: &mut Vec<ResolvedReference>,
) {
    match ty {
        TypeExpr::Reference(r) => {
            let symbol = symbols.get_concept(&r.name).cloned();
            references.push(ResolvedReference {
                reference: Reference {
                    name: r.name.clone(),
                    kind: ReferenceKind::Type,
                    span: r.span,
                },
                symbol,
            });
        }
        TypeExpr::List { element, .. } => {
            let symbol = symbols.get_concept(&element.name).cloned();
            references.push(ResolvedReference {
                reference: Reference {
                    name: element.name.clone(),
                    kind: ReferenceKind::Type,
                    span: element.span,
                },
                symbol,
            });
        }
        TypeExpr::Optional { inner, .. } => {
            let symbol = symbols.get_concept(&inner.name).cloned();
            references.push(ResolvedReference {
                reference: Reference {
                    name: inner.name.clone(),
                    kind: ReferenceKind::Type,
                    span: inner.span,
                },
                symbol,
            });
        }
        TypeExpr::Applied { base, arg, .. } => {
            // Base is typically a generic type like Optional
            let base_symbol = symbols.get_concept(&base.name).cloned();
            references.push(ResolvedReference {
                reference: Reference {
                    name: base.name.clone(),
                    kind: ReferenceKind::Type,
                    span: base.span,
                },
                symbol: base_symbol,
            });

            let arg_symbol = symbols.get_concept(&arg.name).cloned();
            references.push(ResolvedReference {
                reference: Reference {
                    name: arg.name.clone(),
                    kind: ReferenceKind::Type,
                    span: arg.span,
                },
                symbol: arg_symbol,
            });
        }
        TypeExpr::Hole(_) | TypeExpr::OneOf { .. } => {
            // No references to resolve
        }
    }
}

/// Collect references from a behavior definition.
fn collect_behavior_references(
    behavior: &Behavior,
    symbols: &SymbolTable,
    references: &mut Vec<ResolvedReference>,
) {
    // Collect requirement references from implements clause
    for req_id in &behavior.implements {
        let symbol = symbols.get_requirement(&req_id.value).cloned();
        references.push(ResolvedReference {
            reference: Reference {
                name: req_id.value.clone(),
                kind: ReferenceKind::Requirement,
                span: req_id.span,
            },
            symbol,
        });
    }
}

/// Collect references from a task definition.
fn collect_task_references(
    task: &Task,
    symbols: &SymbolTable,
    references: &mut Vec<ResolvedReference>,
) {
    // Collect requirement references
    for req_id in &task.req_refs {
        let symbol = symbols.get_requirement(&req_id.value).cloned();
        references.push(ResolvedReference {
            reference: Reference {
                name: req_id.value.clone(),
                kind: ReferenceKind::Requirement,
                span: req_id.span,
            },
            symbol,
        });
    }
}

/// Resolve a single reference by name.
pub fn resolve<'a>(symbols: &'a SymbolTable, name: &str) -> Option<&'a Symbol> {
    // Try different symbol kinds
    symbols
        .get_requirement(name)
        .or_else(|| symbols.get_task(name))
        .or_else(|| symbols.get_concept(name))
        .or_else(|| symbols.get_behavior(name))
        .or_else(|| symbols.get(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_resolve_requirement_reference() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: First Requirement
Description here.

# Tasks

## TASK-1: Implement REQ-1 [REQ-1]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let resolved = resolve_references(&db, file);

        // Should find the REQ-1 reference in TASK-1
        let req_refs: Vec<_> = resolved
            .references
            .iter()
            .filter(|r| r.reference.kind == ReferenceKind::Requirement)
            .collect();

        assert!(!req_refs.is_empty(), "Should find requirement references");
        assert!(
            req_refs.iter().any(|r| r.symbol.is_some()),
            "REQ-1 reference should be resolved"
        );
    }

    #[test]
    fn test_unresolved_reference() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Tasks

## TASK-1: Implement Missing [REQ-MISSING]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let resolved = resolve_references(&db, file);

        let unresolved: Vec<_> = resolved.unresolved().collect();
        assert!(!unresolved.is_empty(), "Should have unresolved reference");
        assert_eq!(unresolved[0].reference.name, "REQ-MISSING");
    }

    #[test]
    fn test_resolve_type_reference() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept User:
  field name (`String`)

Concept Post:
  field author (`User`)
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let resolved = resolve_references(&db, file);

        // User reference in Post.author should be resolved
        let type_refs: Vec<_> = resolved
            .references
            .iter()
            .filter(|r| r.reference.kind == ReferenceKind::Type)
            .collect();

        // Find the User reference
        let user_ref = type_refs.iter().find(|r| r.reference.name == "User");
        assert!(user_ref.is_some(), "Should find User type reference");
        assert!(
            user_ref.unwrap().symbol.is_some(),
            "User reference should be resolved"
        );
    }
}
