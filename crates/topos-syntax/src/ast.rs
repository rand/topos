//! Typed AST nodes for Topos specifications.
//!
//! This module defines the typed abstract syntax tree that is produced by
//! converting the tree-sitter concrete syntax tree. All nodes include source
//! spans for error reporting and IDE features.

use facet::Facet;

use crate::span::Span;

// ============================================================================
// Top-Level Structures
// ============================================================================

/// A complete Topos source file.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SourceFile {
    /// The spec declaration (e.g., `spec TaskManagement`).
    pub spec: Option<SpecDecl>,
    /// Import statements.
    pub imports: Vec<Import>,
    /// Top-level sections.
    pub sections: Vec<Section>,
    /// Top-level prose (outside sections).
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// A spec declaration: `spec Name`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct SpecDecl {
    /// The spec name.
    pub name: Identifier,
    /// Source span.
    pub span: Span,
}

/// An import statement.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Import {
    /// The import kind.
    pub kind: ImportKind,
    /// Source span.
    pub span: Span,
}

/// Kind of import statement.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum ImportKind {
    /// Import items from a module: `import from "path": Item1, Item2`.
    Items {
        /// Optional source path.
        from: Option<StringLit>,
        /// Items to import.
        items: Vec<ImportItem>,
    },
    /// Import a module as a name: `import "path" as name`.
    Module {
        /// The source path.
        path: StringLit,
        /// The local alias.
        alias: Identifier,
    },
}

/// A single import item.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct ImportItem {
    /// The reference to import.
    pub reference: Reference,
    /// Optional local alias.
    pub alias: Option<Identifier>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Sections
// ============================================================================

/// A top-level section (starts with `#`).
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Section {
    /// Section header text.
    pub header: Prose,
    /// Section contents.
    pub contents: Vec<SectionContent>,
    /// Source span.
    pub span: Span,
}

/// Content that can appear within a section.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum SectionContent {
    /// A requirement definition.
    Requirement(Requirement),
    /// A concept definition.
    Concept(Concept),
    /// A behavior definition.
    Behavior(Behavior),
    /// An invariant definition.
    Invariant(Invariant),
    /// A task definition.
    Task(Task),
    /// An aesthetic block.
    Aesthetic(Aesthetic),
    /// A foreign code block.
    ForeignBlock(ForeignBlock),
    /// A subsection (## that's not a requirement/task).
    Subsection(Subsection),
    /// Prose content.
    Prose(Prose),
}

/// A subsection (## Header without REQ/TASK pattern).
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Subsection {
    /// Header text.
    pub header: Prose,
    /// Body prose.
    pub body: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Requirements
// ============================================================================

/// A requirement definition: `## REQ-ID: Title`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Requirement {
    /// The requirement ID (e.g., `REQ-AUTH-1`).
    pub id: ReqId,
    /// The requirement title.
    pub title: Prose,
    /// EARS clauses (when/the system shall).
    pub ears_clauses: Vec<EarsClause>,
    /// Acceptance criteria.
    pub acceptance: Option<Acceptance>,
    /// Additional prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// A requirement ID (e.g., `REQ-AUTH-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct ReqId {
    /// The full ID string.
    pub value: String,
    /// Source span.
    pub span: Span,
}

/// An EARS clause: `when: ... the system shall: ...`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct EarsClause {
    /// The "when" condition.
    pub when: Prose,
    /// The "the system shall" behavior.
    pub shall: Prose,
    /// Source span.
    pub span: Span,
}

/// Acceptance criteria block.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Acceptance {
    /// The acceptance clauses (given/when/then).
    pub clauses: Vec<AcceptanceClause>,
    /// Source span.
    pub span: Span,
}

/// A single acceptance clause.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct AcceptanceClause {
    /// The clause kind.
    pub kind: AcceptanceKind,
    /// The clause content.
    pub content: Prose,
    /// Source span.
    pub span: Span,
}

/// Kind of acceptance clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum AcceptanceKind {
    /// `given:` clause.
    Given,
    /// `when:` clause.
    When,
    /// `then:` clause.
    Then,
}

// ============================================================================
// Concepts
// ============================================================================

/// A concept definition: `Concept Name:`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Concept {
    /// The concept name.
    pub name: Identifier,
    /// The fields.
    pub fields: Vec<Field>,
    /// Additional prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// A field definition within a concept.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Field {
    /// The field name.
    pub name: Identifier,
    /// The field type (optional).
    pub ty: Option<TypeExpr>,
    /// Field constraints.
    pub constraints: Vec<Constraint>,
    /// Source span.
    pub span: Span,
}

/// A type expression.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum TypeExpr {
    /// A reference to a type: `` `TypeName` ``.
    Reference(Reference),
    /// A typed hole: `[?]` or `[? content]`.
    Hole(TypedHole),
    /// List type: `List of `Type``.
    List { element: Reference, span: Span },
    /// Optional type: `Optional `Type``.
    Optional { inner: Reference, span: Span },
    /// Generic application: `` `Optional` `Type` ``.
    Applied {
        base: Reference,
        arg: Reference,
        span: Span,
    },
    /// Variant type: `one of: A, B, C`.
    OneOf { variants: Vec<Identifier>, span: Span },
}

impl TypeExpr {
    /// Get the span of this type expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Reference(r) => r.span,
            Self::Hole(h) => h.span,
            Self::List { span, .. }
            | Self::Optional { span, .. }
            | Self::Applied { span, .. }
            | Self::OneOf { span, .. } => *span,
        }
    }
}

/// A typed hole: `[?]` or `[? content]`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct TypedHole {
    /// Optional hole content (type signature, name, etc.).
    pub content: Option<String>,
    /// Source span.
    pub span: Span,
}

/// A field constraint.
#[derive(Debug, Clone, PartialEq, Facet)]
#[repr(C)]
pub enum Constraint {
    /// `unique` constraint.
    Unique { span: Span },
    /// `default: value` constraint.
    Default { value: Prose, span: Span },
    /// `derived: expression` constraint.
    Derived { expr: Prose, span: Span },
    /// `invariant: predicate` constraint.
    Invariant { predicate: Prose, span: Span },
    /// `at least N unit` constraint.
    AtLeast {
        count: u64,
        unit: Option<Identifier>,
        span: Span,
    },
}

// ============================================================================
// Behaviors
// ============================================================================

/// A behavior definition: `Behavior Name:`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Behavior {
    /// The behavior name.
    pub name: Identifier,
    /// Requirements this behavior implements.
    pub implements: Vec<ReqId>,
    /// Given clauses.
    pub given: Vec<Prose>,
    /// Returns clause.
    pub returns: Option<Prose>,
    /// Requires (precondition) clauses.
    pub requires: Vec<Prose>,
    /// Ensures (postcondition) clauses.
    pub ensures: Vec<Prose>,
    /// EARS clauses.
    pub ears_clauses: Vec<EarsClause>,
    /// Additional prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Invariants
// ============================================================================

/// An invariant definition: `Invariant Name:`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Invariant {
    /// The invariant name.
    pub name: Identifier,
    /// Quantifiers.
    pub quantifiers: Vec<Quantifier>,
    /// Body prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// A quantifier: `for each x in Collection:`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Quantifier {
    /// The bound variable.
    pub var: Identifier,
    /// The collection being iterated.
    pub collection: Reference,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Tasks
// ============================================================================

/// A task definition: `## TASK-ID: Title [REQ-1, REQ-2]`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Task {
    /// The task ID (e.g., `TASK-AUTH-1`).
    pub id: TaskId,
    /// The task title.
    pub title: Prose,
    /// Referenced requirements.
    pub req_refs: Vec<ReqId>,
    /// Task fields (file, tests, status, etc.).
    pub fields: Vec<TaskField>,
    /// Additional prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// A task ID (e.g., `TASK-AUTH-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct TaskId {
    /// The full ID string.
    pub value: String,
    /// Source span.
    pub span: Span,
}

/// A task field (file, tests, status, etc.).
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct TaskField {
    /// The field kind.
    pub kind: TaskFieldKind,
    /// The field value.
    pub value: Prose,
    /// Source span.
    pub span: Span,
}

/// Kind of task field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
#[repr(u8)]
pub enum TaskFieldKind {
    /// `file:` field.
    File,
    /// `tests:` field.
    Tests,
    /// `depends:` field.
    Depends,
    /// `status:` field.
    Status,
    /// `evidence:` field.
    Evidence,
    /// `context:` field.
    Context,
}

// ============================================================================
// Aesthetics
// ============================================================================

/// An aesthetic block: `Aesthetic Name:`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Aesthetic {
    /// The aesthetic name.
    pub name: Identifier,
    /// Aesthetic fields.
    pub fields: Vec<AestheticField>,
    /// Additional prose.
    pub prose: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

/// An aesthetic field: `name: [~] value`.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct AestheticField {
    /// The field name.
    pub name: Identifier,
    /// Whether this is a soft constraint (`[~]`).
    pub is_soft: bool,
    /// The field value.
    pub value: Prose,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Foreign Blocks
// ============================================================================

/// A foreign code block: ```language ... ```.
#[derive(Debug, Clone, PartialEq, Facet)]
pub struct ForeignBlock {
    /// The language identifier (e.g., "typespec", "cue").
    pub language: String,
    /// The block content.
    pub content: Vec<Prose>,
    /// Source span.
    pub span: Span,
}

// ============================================================================
// Primitives
// ============================================================================

/// An identifier (alphanumeric name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct Identifier {
    /// The identifier text.
    pub value: String,
    /// Source span.
    pub span: Span,
}

impl Identifier {
    /// Create a new identifier.
    #[must_use]
    pub fn new(value: impl Into<String>, span: Span) -> Self {
        Self {
            value: value.into(),
            span,
        }
    }
}

/// A reference to a concept/type: `` `Name` ``.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct Reference {
    /// The referenced name.
    pub name: String,
    /// Source span.
    pub span: Span,
}

impl Reference {
    /// Create a new reference.
    #[must_use]
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

/// A string literal: `"..."`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct StringLit {
    /// The string value (without quotes).
    pub value: String,
    /// Source span.
    pub span: Span,
}

/// Prose text (free-form content).
#[derive(Debug, Clone, PartialEq, Eq, Facet)]
pub struct Prose {
    /// The prose text.
    pub text: String,
    /// Source span.
    pub span: Span,
}

impl Prose {
    /// Create new prose.
    #[must_use]
    pub fn new(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

// ============================================================================
// Visitor Trait
// ============================================================================

/// Trait for visiting AST nodes.
pub trait Visitor {
    /// Visit a source file.
    fn visit_source_file(&mut self, _file: &SourceFile) {}
    /// Visit a section.
    fn visit_section(&mut self, _section: &Section) {}
    /// Visit a requirement.
    fn visit_requirement(&mut self, _req: &Requirement) {}
    /// Visit a concept.
    fn visit_concept(&mut self, _concept: &Concept) {}
    /// Visit a behavior.
    fn visit_behavior(&mut self, _behavior: &Behavior) {}
    /// Visit an invariant.
    fn visit_invariant(&mut self, _invariant: &Invariant) {}
    /// Visit a task.
    fn visit_task(&mut self, _task: &Task) {}
    /// Visit an aesthetic block.
    fn visit_aesthetic(&mut self, _aesthetic: &Aesthetic) {}
    /// Visit a typed hole.
    fn visit_hole(&mut self, _hole: &TypedHole) {}
}

/// Walk an AST with a visitor.
pub fn walk<V: Visitor>(visitor: &mut V, file: &SourceFile) {
    visitor.visit_source_file(file);
    for section in &file.sections {
        visitor.visit_section(section);
        for content in &section.contents {
            match content {
                SectionContent::Requirement(r) => visitor.visit_requirement(r),
                SectionContent::Concept(c) => visitor.visit_concept(c),
                SectionContent::Behavior(b) => visitor.visit_behavior(b),
                SectionContent::Invariant(i) => visitor.visit_invariant(i),
                SectionContent::Task(t) => visitor.visit_task(t),
                SectionContent::Aesthetic(a) => visitor.visit_aesthetic(a),
                SectionContent::ForeignBlock(_) | SectionContent::Subsection(_) | SectionContent::Prose(_) => {}
            }
        }
    }
}
