//! Unified polyglot symbol resolution.
//!
//! This module provides a unified view of symbols from both Topos
//! specifications and embedded foreign code blocks.

use std::sync::Arc;

use topos_syntax::Span;

use crate::db::{self, Db};
use crate::foreign::{foreign_symbols, ForeignSymbol, ForeignSymbolKind};
use crate::symbols::{symbols, Symbol, SymbolKind};

/// A unified symbol that can represent either a Topos symbol or a foreign symbol.
#[derive(Debug, Clone)]
pub enum UnifiedSymbol {
    /// A native Topos symbol (requirement, task, concept, etc.).
    Topos(Symbol),
    /// A foreign symbol from an embedded code block.
    Foreign(ForeignSymbol),
}

impl UnifiedSymbol {
    /// Get the symbol name.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Topos(s) => &s.name,
            Self::Foreign(s) => &s.name,
        }
    }

    /// Get the symbol's source span.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Topos(s) => s.span,
            Self::Foreign(s) => s.span,
        }
    }

    /// Check if this is a Topos symbol.
    #[must_use]
    pub fn is_topos(&self) -> bool {
        matches!(self, Self::Topos(_))
    }

    /// Check if this is a foreign symbol.
    #[must_use]
    pub fn is_foreign(&self) -> bool {
        matches!(self, Self::Foreign(_))
    }

    /// Get a human-readable kind label.
    #[must_use]
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Topos(s) => match s.kind {
                SymbolKind::Requirement => "requirement",
                SymbolKind::Task => "task",
                SymbolKind::Concept => "concept",
                SymbolKind::Behavior => "behavior",
                SymbolKind::Invariant => "invariant",
                SymbolKind::Field => "field",
            },
            Self::Foreign(s) => s.kind.label(),
        }
    }

    /// Get hover documentation for this symbol.
    #[must_use]
    pub fn hover_docs(&self) -> String {
        match self {
            Self::Topos(s) => {
                let mut doc = format!("**{}** `{}`", self.kind_label(), s.name);
                if let Some(title) = &s.title {
                    doc.push_str(&format!("\n\n{title}"));
                }
                if let Some(status) = &s.status {
                    doc.push_str(&format!("\n\nStatus: {status}"));
                }
                doc
            }
            Self::Foreign(s) => {
                let mut doc = format!(
                    "**{}** `{}` ({})",
                    s.kind.label(),
                    s.name,
                    s.language
                );
                doc.push_str(&format!("\n\n```{}\n{}\n```", s.language, s.declaration));
                doc
            }
        }
    }
}

/// Unified symbol table combining Topos and foreign symbols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedSymbolTable {
    /// All Topos symbols.
    topos_symbols: Arc<crate::symbols::SymbolTable>,
    /// All foreign symbols.
    foreign_symbols: Arc<crate::foreign::ForeignSymbols>,
}

impl UnifiedSymbolTable {
    /// Look up a symbol by name.
    ///
    /// Resolution order:
    /// 1. Local Topos symbols
    /// 2. Foreign symbols (same file)
    #[must_use]
    pub fn get(&self, name: &str) -> Option<UnifiedSymbol> {
        // First check Topos symbols
        if let Some(sym) = self.topos_symbols.get(name) {
            return Some(UnifiedSymbol::Topos(sym.clone()));
        }

        // Then check foreign symbols
        if let Some(sym) = self.foreign_symbols.get(name) {
            return Some(UnifiedSymbol::Foreign(sym.clone()));
        }

        None
    }

    /// Look up a symbol as a type reference.
    ///
    /// For backtick references like `User`, this checks:
    /// 1. Topos concepts
    /// 2. Foreign types (models, interfaces, enums, etc.)
    #[must_use]
    pub fn get_type(&self, name: &str) -> Option<UnifiedSymbol> {
        // First check Topos concepts
        if let Some(sym) = self.topos_symbols.get_concept(name) {
            return Some(UnifiedSymbol::Topos(sym.clone()));
        }

        // Then check foreign type-like symbols
        if let Some(sym) = self.foreign_symbols.get(name) {
            match sym.kind {
                ForeignSymbolKind::Model
                | ForeignSymbolKind::Interface
                | ForeignSymbolKind::TypeAlias
                | ForeignSymbolKind::Enum
                | ForeignSymbolKind::Union
                | ForeignSymbolKind::Schema => {
                    return Some(UnifiedSymbol::Foreign(sym.clone()));
                }
                _ => {}
            }
        }

        None
    }

    /// Get all symbols (both Topos and foreign).
    pub fn all_symbols(&self) -> impl Iterator<Item = UnifiedSymbol> + '_ {
        let topos_iter = self
            .topos_symbols
            .symbols
            .values()
            .cloned()
            .map(UnifiedSymbol::Topos);
        let foreign_iter = self
            .foreign_symbols
            .symbols
            .iter()
            .cloned()
            .map(UnifiedSymbol::Foreign);
        topos_iter.chain(foreign_iter)
    }

    /// Get all foreign symbols.
    pub fn foreign(&self) -> &crate::foreign::ForeignSymbols {
        &self.foreign_symbols
    }

    /// Get all Topos symbols.
    pub fn topos(&self) -> &crate::symbols::SymbolTable {
        &self.topos_symbols
    }

    /// Get all type-like symbols for completion.
    pub fn all_types(&self) -> impl Iterator<Item = UnifiedSymbol> + '_ {
        let topos_concepts = self
            .topos_symbols
            .concepts
            .values()
            .cloned()
            .map(UnifiedSymbol::Topos);

        let foreign_types = self
            .foreign_symbols
            .symbols
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    ForeignSymbolKind::Model
                        | ForeignSymbolKind::Interface
                        | ForeignSymbolKind::TypeAlias
                        | ForeignSymbolKind::Enum
                        | ForeignSymbolKind::Union
                        | ForeignSymbolKind::Schema
                )
            })
            .cloned()
            .map(UnifiedSymbol::Foreign);

        topos_concepts.chain(foreign_types)
    }
}

/// Build a unified symbol table for a source file.
#[salsa::tracked]
pub fn unified_symbols(db: &dyn Db, file: db::SourceFile) -> Arc<UnifiedSymbolTable> {
    let topos_symbols = symbols(db, file);
    let foreign_syms = foreign_symbols(db, file);

    Arc::new(UnifiedSymbolTable {
        topos_symbols,
        foreign_symbols: foreign_syms,
    })
}

/// Resolve a reference to its symbol.
///
/// Returns the resolved symbol if found, or None if unresolved.
#[must_use]
pub fn resolve_reference(table: &UnifiedSymbolTable, name: &str) -> Option<UnifiedSymbol> {
    table.get(name)
}

/// Resolve a type reference (backtick reference).
#[must_use]
pub fn resolve_type_reference(table: &UnifiedSymbolTable, name: &str) -> Option<UnifiedSymbol> {
    table.get_type(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_unified_resolution_topos_first() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept User:
  field id

```typespec
model Order {
  id: string;
}
```
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        // Topos concept should be found
        let user = table.get("User");
        assert!(user.is_some());
        assert!(user.unwrap().is_topos());

        // Foreign symbol should also be found
        let order = table.get("Order");
        assert!(order.is_some());
        assert!(order.unwrap().is_foreign());
    }

    #[test]
    fn test_type_resolution() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept LocalType:
  field value

```typespec
model ForeignModel {}
interface ForeignInterface {}
enum ForeignEnum {}
```
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        // All types should resolve
        assert!(table.get_type("LocalType").is_some());
        assert!(table.get_type("ForeignModel").is_some());
        assert!(table.get_type("ForeignInterface").is_some());
        assert!(table.get_type("ForeignEnum").is_some());
    }

    #[test]
    fn test_hover_docs_topos() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: User Authentication
Users must be able to log in.
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        let req = table.get("REQ-1").unwrap();
        let docs = req.hover_docs();
        assert!(docs.contains("requirement"));
        assert!(docs.contains("REQ-1"));
        assert!(docs.contains("User Authentication"));
    }

    #[test]
    fn test_hover_docs_foreign() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Types

```typespec
model User {
  id: string;
}
```
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        let user = table.get("User").unwrap();
        let docs = user.hover_docs();
        assert!(docs.contains("model"));
        assert!(docs.contains("User"));
        assert!(docs.contains("typespec"));
    }

    #[test]
    fn test_all_types() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept A:
  field x

Concept B:
  field y

```typespec
model C {}
interface D {}
op someOp(): void;
```
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        let types: Vec<_> = table.all_types().collect();
        let names: Vec<_> = types.iter().map(|t| t.name()).collect();

        assert!(names.contains(&"A"));
        assert!(names.contains(&"B"));
        assert!(names.contains(&"C"));
        assert!(names.contains(&"D"));
        // Operations are not types
        assert!(!names.contains(&"someOp"));
    }

    #[test]
    fn test_kind_labels() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: Test
Description.

# Concepts

Concept User:
  field id

# Tasks

## TASK-1: Do thing [REQ-1]
status: pending

# Types

```typespec
model Order {}
interface Api {}
enum Status {}
```
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = unified_symbols(&db, file);

        assert_eq!(table.get("REQ-1").unwrap().kind_label(), "requirement");
        assert_eq!(table.get("User").unwrap().kind_label(), "concept");
        assert_eq!(table.get("TASK-1").unwrap().kind_label(), "task");
        assert_eq!(table.get("Order").unwrap().kind_label(), "model");
        assert_eq!(table.get("Api").unwrap().kind_label(), "interface");
        assert_eq!(table.get("Status").unwrap().kind_label(), "enum");
    }
}
