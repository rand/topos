//! Symbol table construction for Topos specifications.
//!
//! This module extracts defined symbols from the AST and builds
//! a symbol table for reference resolution.

use std::collections::HashMap;
use std::sync::Arc;

use topos_syntax::{
    Behavior, Concept, Invariant, Requirement, SectionContent, SourceFile, Span, Task,
};

use crate::db::{self, Db};

/// A symbol in the symbol table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The symbol name/identifier.
    pub name: String,
    /// The kind of symbol.
    pub kind: SymbolKind,
    /// Human-readable title (for requirements, tasks).
    pub title: Option<String>,
    /// Status (for tasks: pending, in_progress, done).
    pub status: Option<String>,
    /// Source location where the symbol is defined.
    pub span: Span,
}

/// Kinds of symbols in Topos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// A requirement (REQ-*).
    Requirement,
    /// A task (TASK-*).
    Task,
    /// A concept definition.
    Concept,
    /// A behavior definition.
    Behavior,
    /// An invariant definition.
    Invariant,
    /// A field within a concept.
    Field,
}

/// A symbol table containing all defined symbols.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SymbolTable {
    /// All symbols indexed by name.
    pub symbols: HashMap<String, Symbol>,
    /// Requirements by ID.
    pub requirements: HashMap<String, Symbol>,
    /// Tasks by ID.
    pub tasks: HashMap<String, Symbol>,
    /// Concepts by name.
    pub concepts: HashMap<String, Symbol>,
    /// Behaviors by name.
    pub behaviors: HashMap<String, Symbol>,
    /// Invariants by name.
    pub invariants: HashMap<String, Symbol>,
}

impl SymbolTable {
    /// Create a new empty symbol table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the table.
    pub fn add(&mut self, symbol: Symbol) {
        let name = symbol.name.clone();
        match symbol.kind {
            SymbolKind::Requirement => {
                self.requirements.insert(name.clone(), symbol.clone());
            }
            SymbolKind::Task => {
                self.tasks.insert(name.clone(), symbol.clone());
            }
            SymbolKind::Concept => {
                self.concepts.insert(name.clone(), symbol.clone());
            }
            SymbolKind::Behavior => {
                self.behaviors.insert(name.clone(), symbol.clone());
            }
            SymbolKind::Invariant => {
                self.invariants.insert(name.clone(), symbol.clone());
            }
            SymbolKind::Field => {
                // Fields are added to the general symbols map only
            }
        }
        self.symbols.insert(name, symbol);
    }

    /// Look up a symbol by name.
    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Look up a requirement by ID.
    pub fn get_requirement(&self, id: &str) -> Option<&Symbol> {
        self.requirements.get(id)
    }

    /// Look up a task by ID.
    pub fn get_task(&self, id: &str) -> Option<&Symbol> {
        self.tasks.get(id)
    }

    /// Look up a concept by name.
    pub fn get_concept(&self, name: &str) -> Option<&Symbol> {
        self.concepts.get(name)
    }

    /// Look up a behavior by name.
    pub fn get_behavior(&self, name: &str) -> Option<&Symbol> {
        self.behaviors.get(name)
    }
}

/// Build a symbol table from a source file.
#[salsa::tracked]
pub fn symbols(db: &dyn Db, file: db::SourceFile) -> Arc<SymbolTable> {
    let ast = db::parse(db, file);
    let mut table = SymbolTable::new();

    collect_symbols(&ast, &mut table);

    Arc::new(table)
}

/// Collect symbols from an AST.
fn collect_symbols(ast: &SourceFile, table: &mut SymbolTable) {
    for section in &ast.sections {
        for content in &section.contents {
            match content {
                SectionContent::Requirement(req) => {
                    add_requirement(req, table);
                }
                SectionContent::Task(task) => {
                    add_task(task, table);
                }
                SectionContent::Concept(concept) => {
                    add_concept(concept, table);
                }
                SectionContent::Behavior(behavior) => {
                    add_behavior(behavior, table);
                }
                SectionContent::Invariant(invariant) => {
                    add_invariant(invariant, table);
                }
                _ => {}
            }
        }
    }
}

fn add_requirement(req: &Requirement, table: &mut SymbolTable) {
    table.add(Symbol {
        name: req.id.value.clone(),
        kind: SymbolKind::Requirement,
        title: Some(req.title.text.trim().to_string()),
        status: None,
        span: req.span,
    });
}

fn add_task(task: &Task, table: &mut SymbolTable) {
    // Extract status from task fields
    let status = task
        .fields
        .iter()
        .find(|f| matches!(f.kind, topos_syntax::TaskFieldKind::Status))
        .map(|f| f.value.text.trim().to_string());

    table.add(Symbol {
        name: task.id.value.clone(),
        kind: SymbolKind::Task,
        title: Some(task.title.text.trim().to_string()),
        status,
        span: task.span,
    });
}

fn add_concept(concept: &Concept, table: &mut SymbolTable) {
    table.add(Symbol {
        name: concept.name.value.clone(),
        kind: SymbolKind::Concept,
        title: None,
        status: None,
        span: concept.span,
    });

    // Also add fields
    for field in &concept.fields {
        table.add(Symbol {
            name: format!("{}.{}", concept.name.value, field.name.value),
            kind: SymbolKind::Field,
            title: None,
            status: None,
            span: field.span,
        });
    }
}

fn add_behavior(behavior: &Behavior, table: &mut SymbolTable) {
    table.add(Symbol {
        name: behavior.name.value.clone(),
        kind: SymbolKind::Behavior,
        title: None,
        status: None,
        span: behavior.span,
    });
}

fn add_invariant(invariant: &Invariant, table: &mut SymbolTable) {
    table.add(Symbol {
        name: invariant.name.value.clone(),
        kind: SymbolKind::Invariant,
        title: None,
        status: None,
        span: invariant.span,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_empty_symbols() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_file("test.tps".to_string(), "spec Test\n".to_string());
        let table = symbols(&db, file);
        assert!(table.symbols.is_empty());
    }

    #[test]
    fn test_requirement_symbol() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: Test Requirement
Description here.
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = symbols(&db, file);

        assert!(table.get_requirement("REQ-1").is_some());
        let req = table.get_requirement("REQ-1").unwrap();
        assert_eq!(req.kind, SymbolKind::Requirement);
    }

    #[test]
    fn test_concept_symbol() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept User:
  field name (`String`)
  field email (`String`)
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = symbols(&db, file);

        assert!(table.get_concept("User").is_some());
        let concept = table.get_concept("User").unwrap();
        assert_eq!(concept.kind, SymbolKind::Concept);

        // Check fields are added
        assert!(table.get("User.name").is_some());
        assert!(table.get("User.email").is_some());
    }

    #[test]
    fn test_task_symbol() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Tasks

## TASK-1: Implement Feature [REQ-1]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let table = symbols(&db, file);

        assert!(table.get_task("TASK-1").is_some());
    }
}
