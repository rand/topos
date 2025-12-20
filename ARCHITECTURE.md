# Topos Architecture

**Version**: 2.0.0  
**Date**: December 2025  
**Status**: Technical Design

---

## Executive Summary

This document describes the architecture of Topos, a semantic contract language toolchain built in Rust. The architecture leverages **facet.rs** for reflection-based operations (serialization, diffing, pretty-printing), **Salsa** for incremental computation, and **rmcp** for AI integration via the Model Context Protocol.

---

## System Architecture

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              Topos Toolchain                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Core Library (Rust)                             │ │
│  │                                                                         │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │ │
│  │  │ tree-sitter  │  │   Facet-     │  │   Salsa DB   │  │  Facet-    │  │ │
│  │  │   Grammar    │─▶│   Powered    │─▶│  (Incremental│─▶│  Diff      │  │ │
│  │  │   Parser     │  │   AST        │  │   Cache)     │  │  Engine    │  │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  └────────────┘  │ │
│  │         │                 │                 │                │         │ │
│  │         │    #[derive(Facet)] on all types  │                │         │ │
│  │         │                 │                 │                │         │ │
│  │         ▼                 ▼                 ▼                ▼         │ │
│  │  ┌──────────────────────────────────────────────────────────────────┐ │ │
│  │  │                 Unified Type System (facet.rs)                    │ │ │
│  │  │                                                                   │ │ │
│  │  │  • Peek: Read-only inspection (hover, debugging)                 │ │ │
│  │  │  • Partial: Incremental construction (parsing)                   │ │ │
│  │  │  • Diffing: Structural comparison (spec↔code sync)               │ │ │
│  │  │  • Pretty: Colored output (diagnostics, reports)                 │ │ │
│  │  │  • JSON/YAML/TOML: Serialization (export, MCP)                   │ │ │
│  │  └──────────────────────────────────────────────────────────────────┘ │ │
│  │                                                                         │ │
│  └─────────────────────────────────┬───────────────────────────────────────┘ │
│                                    │                                         │
│        ┌───────────────────────────┼───────────────────────────┐            │
│        │                           │                           │            │
│        ▼                           ▼                           ▼            │
│ ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐     │
│ │   LSP Server    │       │   MCP Server    │       │      CLI        │     │
│ │ (tower-lsp-     │       │    (rmcp)       │       │    (clap)       │     │
│ │   server)       │       │                 │       │                 │     │
│ │                 │       │ • create_spec   │       │ • topos check   │     │
│ │ • Diagnostics   │       │ • generate_code │       │ • topos format  │     │
│ │ • Hover (Peek)  │       │ • extract_spec  │       │ • topos trace   │     │
│ │ • Go-to-def     │       │ • complete_hole │       │ • topos export  │     │
│ │ • Find refs     │       │ • analyze_spec  │       │ • topos lsp     │     │
│ │ • Completions   │       │ • trace_req     │       │ • topos mcp     │     │
│ │ • Code actions  │       │                 │       │                 │     │
│ │ • Diff preview  │       │                 │       │                 │     │
│ └─────────────────┘       └─────────────────┘       └─────────────────┘     │
│        │                           │                           │            │
│        ▼                           ▼                           ▼            │
│ ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐     │
│ │  VS Code Ext    │       │  Claude/LLM     │       │   CI/CD         │     │
│ │  (TypeScript)   │       │  Integration    │       │   Integration   │     │
│ └─────────────────┘       └─────────────────┘       └─────────────────┘     │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Design Decisions

### 1. facet.rs as Foundation

**Decision**: All Topos types derive `Facet` instead of multiple serde/debug/eq traits.

**Rationale**:
- **Single derive**: `#[derive(Facet)]` replaces `Serialize`, `Deserialize`, `Debug`, `PartialEq`, `Clone` 
- **Fast compilation**: Uses `unsynn` instead of `syn`, dramatically reducing compile times
- **Runtime flexibility**: Can inspect and diff any type at runtime without compile-time monomorphization
- **Structural diffing**: `facet-diff` enables spec↔code drift detection natively
- **Pretty printing**: `facet-pretty` provides colored, redacted output for diagnostics

**Trade-off**: Slight runtime overhead compared to monomorphized serde, but acceptable for tooling use cases where flexibility matters more than microseconds.

```rust
use facet::Facet;

// All AST nodes derive Facet
#[derive(Facet)]
pub struct Concept {
    pub name: Identifier,
    pub doc: Option<DocString>,
    #[facet(skip_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Facet)]
pub struct TypedHole {
    pub id: HoleId,
    #[facet(sensitive)]  // Redacted in logs
    pub name: Option<String>,
    pub signature: Option<HoleSignature>,
    pub constraints: Vec<HoleConstraint>,
    pub span: Span,
}
```

### 2. Salsa for Incremental Computation

**Decision**: Use Salsa 0.18+ with durability optimization for all derived computations.

**Rationale**:
- Proven at scale in rust-analyzer
- Durability system optimizes for the common case (editing user code, not stdlib)
- Automatic dependency tracking and early cutoff
- Supports cancellation for responsive UI

```rust
#[salsa::db]
pub trait ToposDatabase: salsa::Database {
    // Inputs (volatile - change frequently)
    #[salsa::input]
    fn file_text(&self, file: FileId) -> Arc<str>;
    
    // Derived (memoized)
    #[salsa::tracked]
    fn parse_file(&self, file: FileId) -> Arc<ParseResult>;
    
    #[salsa::tracked]
    fn resolve_imports(&self, file: FileId) -> Arc<ImportMap>;
    
    #[salsa::tracked]
    fn analyze_file(&self, file: FileId) -> Arc<AnalysisResult>;
    
    // Cross-file (uses durability)
    #[salsa::tracked]
    fn workspace_diagnostics(&self) -> Arc<Vec<Diagnostic>>;
}
```

### 3. tree-sitter for Parsing

**Decision**: tree-sitter grammar with external scanner for indentation.

**Rationale**:
- Sub-millisecond incremental reparsing essential for LSP
- Robust error recovery (always produces a tree)
- WASM build enables browser-based tooling
- Mature ecosystem with syntax highlighting queries

### 4. rmcp for MCP Integration

**Decision**: Use the official Rust MCP SDK (rmcp 0.8+) for AI integration.

**Rationale**:
- Official SDK from `modelcontextprotocol` organization
- Supports MCP 2025-06-18 specification
- tokio-native async runtime
- Clean macro-based tool definition

```rust
use rmcp::{tool, ServerBuilder, ToolHandler};

#[tool(
    name = "create_spec",
    description = "Generate a Topos specification from natural language intent"
)]
async fn create_spec(
    #[arg(description = "Natural language description of the requirement")]
    intent: String,
    #[arg(description = "Target domain (users, orders, etc.)")]
    domain: Option<String>,
) -> Result<SpecResult, ToolError> {
    // Implementation
}
```

### 5. tower-lsp-server for LSP

**Decision**: Use the community fork `tower-lsp-server` 0.1+ instead of `tower-lsp` 0.20.

**Rationale**:
- Active community maintenance
- Uses `lsp-types` 0.97 (latest)
- Removed `#[async_trait]` for cleaner trait impls
- Better error handling with `fluent_uri`

---

## Crate Structure

```
topos/
├── Cargo.toml                    # Workspace root
├── tree-sitter-topos/           # Grammar (JavaScript + Rust bindings)
│   ├── grammar.js
│   ├── src/
│   │   ├── scanner.c            # External scanner for indentation
│   │   └── parser.c             # Generated
│   └── queries/
│       ├── highlights.scm
│       └── injections.scm
│
├── crates/
│   ├── topos-syntax/            # Parsing and AST
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ast.rs           # AST types (all #[derive(Facet)])
│   │   │   ├── parser.rs        # tree-sitter → AST conversion
│   │   │   └── spans.rs         # Source location tracking
│   │   └── Cargo.toml
│   │
│   ├── topos-analysis/          # Semantic analysis
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── db.rs            # Salsa database
│   │   │   ├── resolve.rs       # Name resolution
│   │   │   ├── holes.rs         # Typed hole analysis
│   │   │   ├── types.rs         # Type checking
│   │   │   └── traceability.rs  # Requirement tracing
│   │   └── Cargo.toml
│   │
│   ├── topos-diff/              # Spec↔Code synchronization
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── extract.rs       # Extract model from code
│   │   │   ├── compare.rs       # Structural diff (facet-diff)
│   │   │   └── reconcile.rs     # Generate patches
│   │   └── Cargo.toml
│   │
│   ├── topos-context/           # Context compilation
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── compiler.rs      # Context compilation logic
│   │   │   ├── pruning.rs       # Relevance-based pruning
│   │   │   ├── formats/
│   │   │   │   ├── cursor.rs    # .cursorrules format
│   │   │   │   ├── cline.rs     # .clinerules format
│   │   │   │   ├── markdown.rs  # Plain markdown
│   │   │   │   └── json.rs      # Structured JSON
│   │   │   └── templates/
│   │   │       └── *.hbs        # Handlebars templates
│   │   └── Cargo.toml
│   │
│   ├── topos-lsp/               # Language server
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── server.rs        # tower-lsp-server impl
│   │   │   ├── handlers/
│   │   │   │   ├── hover.rs     # Uses Peek for inspection
│   │   │   │   ├── completion.rs
│   │   │   │   ├── goto.rs
│   │   │   │   ├── references.rs
│   │   │   │   └── actions.rs
│   │   │   └── capabilities.rs
│   │   └── Cargo.toml
│   │
│   ├── topos-mcp/               # MCP server
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── server.rs        # rmcp server impl
│   │   │   ├── sandbox.rs       # Security sandboxing
│   │   │   ├── tools/
│   │   │   │   ├── create_spec.rs
│   │   │   │   ├── generate_code.rs
│   │   │   │   ├── extract_spec.rs
│   │   │   │   ├── complete_hole.rs
│   │   │   │   ├── analyze_spec.rs
│   │   │   │   ├── trace_requirement.rs
│   │   │   │   └── compile_context.rs  # Context compilation
│   │   │   └── sync.rs          # Bidirectional sync engine (V2)
│   │   └── Cargo.toml
│   │
│   └── topos-cli/               # Command-line interface
│       ├── src/
│       │   ├── main.rs
│       │   └── commands/
│       │       ├── check.rs
│       │       ├── format.rs
│       │       ├── trace.rs
│       │       ├── context.rs   # Context compilation
│       │       ├── drift.rs     # Drift detection
│       │       ├── export.rs
│       │       ├── lsp.rs
│       │       └── mcp.rs
│       └── Cargo.toml
│
└── editors/
    └── vscode/                  # VS Code extension
        ├── package.json
        ├── src/
        │   └── extension.ts
        └── syntaxes/
            └── topos.tmLanguage.json
```

---

## Key Subsystems

### AST with facet.rs

All AST nodes derive `Facet`, enabling unified operations:

```rust
// crates/topos-syntax/src/ast.rs

use facet::Facet;
use std::sync::Arc;

#[derive(Debug, Clone, Facet)]
pub struct SourceFile {
    pub spec_declaration: Option<SpecDeclaration>,
    pub sections: Vec<Section>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct SpecDeclaration {
    pub name: Identifier,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub enum Section {
    Principles(PrinciplesSection),
    Requirements(RequirementsSection),
    Design(DesignSection),
    Concepts(ConceptsSection),
    Behaviors(BehaviorsSection),
    Tasks(TasksSection),
}

#[derive(Debug, Clone, Facet)]
pub struct Concept {
    pub name: Identifier,
    #[facet(skip_if = "Option::is_none")]
    pub doc: Option<DocString>,
    #[facet(skip_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    #[facet(skip_if = "Option::is_none")]
    pub one_of: Option<OneOf>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct TypedHole {
    pub id: HoleId,
    #[facet(skip_if = "Option::is_none")]
    pub name: Option<String>,
    #[facet(skip_if = "Option::is_none")]
    pub signature: Option<HoleSignature>,
    #[facet(skip_if = "Vec::is_empty")]
    pub constraints: Vec<HoleConstraint>,
    #[facet(skip_if = "Vec::is_empty")]
    pub involving: Vec<SymbolId>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct HoleSignature {
    #[facet(skip_if = "Option::is_none")]
    pub input: Option<TypeExpr>,
    #[facet(skip_if = "Option::is_none")]
    pub output: Option<TypeExpr>,
}
```

### Salsa Database

```rust
// crates/topos-analysis/src/db.rs

use salsa::Durability;
use std::sync::Arc;

#[salsa::db]
pub trait ToposDatabase: salsa::Database {
    // === INPUTS (volatile) ===
    
    #[salsa::input]
    fn file_text(&self, file: FileId) -> Arc<str>;
    
    #[salsa::input]
    fn file_path(&self, file: FileId) -> Arc<Path>;
    
    // === PARSING ===
    
    #[salsa::tracked]
    fn parse(&self, file: FileId) -> Arc<ParseResult>;
    
    #[salsa::tracked]
    fn ast(&self, file: FileId) -> Arc<SourceFile>;
    
    // === RESOLUTION ===
    
    #[salsa::tracked]
    fn imports(&self, file: FileId) -> Arc<ImportMap>;
    
    #[salsa::tracked]
    fn exports(&self, file: FileId) -> Arc<ExportMap>;
    
    #[salsa::tracked]
    fn resolve_reference(&self, file: FileId, ref_: Reference) -> Option<Definition>;
    
    // === ANALYSIS ===
    
    #[salsa::tracked]
    fn file_diagnostics(&self, file: FileId) -> Arc<Vec<Diagnostic>>;
    
    #[salsa::tracked]
    fn hole_analysis(&self, file: FileId) -> Arc<HoleAnalysis>;
    
    #[salsa::tracked]
    fn traceability(&self, file: FileId) -> Arc<TraceabilityMap>;
    
    // === WORKSPACE ===
    
    #[salsa::tracked]
    fn workspace_files(&self) -> Arc<Vec<FileId>>;
    
    #[salsa::tracked]
    fn workspace_diagnostics(&self) -> Arc<Vec<Diagnostic>>;
}

#[salsa::db]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for RootDatabase {}

impl ToposDatabase for RootDatabase {}

impl RootDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set standard library files with high durability (rarely change)
    pub fn set_stdlib(&mut self, files: Vec<(FileId, Arc<str>)>) {
        for (file, text) in files {
            self.set_file_text_with_durability(
                file, 
                text, 
                Durability::HIGH
            );
        }
    }
}
```

### Spec↔Code Diffing with facet-diff

```rust
// crates/topos-diff/src/compare.rs

use facet::Facet;
use facet_reflect::{Peek, check_same_report, SameReport};

/// Model extracted from either spec or code
#[derive(Debug, Clone, Facet)]
pub struct DomainModel {
    pub name: String,
    pub concepts: Vec<ConceptModel>,
    pub behaviors: Vec<BehaviorModel>,
    pub types: Vec<TypeModel>,
}

#[derive(Debug, Clone, Facet)]
pub struct ConceptModel {
    pub name: String,
    pub fields: Vec<FieldModel>,
    pub invariants: Vec<String>,
}

/// Compare spec model against code model
pub fn compare_models(
    spec_model: &DomainModel,
    code_model: &DomainModel,
) -> ComparisonResult {
    match check_same_report(spec_model, code_model) {
        SameReport::Same => ComparisonResult::InSync,
        SameReport::Different(report) => {
            ComparisonResult::Drift(DriftReport {
                summary: report.render_plain_json(),
                changes: extract_changes(&report),
            })
        }
        SameReport::Opaque { type_name } => {
            ComparisonResult::Error(format!("Cannot compare opaque type: {}", type_name))
        }
    }
}

#[derive(Debug)]
pub enum ComparisonResult {
    InSync,
    Drift(DriftReport),
    Error(String),
}

#[derive(Debug)]
pub struct DriftReport {
    pub summary: String,
    pub changes: Vec<DriftChange>,
}

#[derive(Debug)]
pub enum DriftChange {
    ConceptAdded { name: String },
    ConceptRemoved { name: String },
    FieldAdded { concept: String, field: String },
    FieldRemoved { concept: String, field: String },
    FieldTypeChanged { concept: String, field: String, from: String, to: String },
    BehaviorSignatureChanged { name: String, diff: String },
}

fn extract_changes(report: &facet_reflect::DiffReport) -> Vec<DriftChange> {
    // Parse the facet diff report into typed changes
    // This enables actionable suggestions in the LSP
    todo!()
}
```

### MCP Server with rmcp

```rust
// crates/topos-mcp/src/server.rs

use rmcp::{ServerBuilder, tool, ToolHandler, ToolResult};
use facet_json::ToJson;

pub struct ToposMcpServer {
    db: Arc<RootDatabase>,
}

impl ToposMcpServer {
    pub fn new(db: Arc<RootDatabase>) -> Self {
        Self { db }
    }
    
    pub async fn run(self) -> Result<(), rmcp::Error> {
        ServerBuilder::new("topos-mcp", "1.0.0")
            .with_tool(CreateSpec)
            .with_tool(GenerateCode)
            .with_tool(ExtractSpec)
            .with_tool(CompleteHole)
            .with_tool(AnalyzeSpec)
            .with_tool(TraceRequirement)
            .build()
            .run_stdio()
            .await
    }
}

struct CreateSpec;

#[tool(
    name = "create_spec",
    description = "Generate a Topos specification from natural language intent"
)]
impl ToolHandler for CreateSpec {
    async fn call(
        &self,
        #[arg(description = "Natural language description")]
        intent: String,
        #[arg(description = "Target domain")]
        domain: Option<String>,
        #[arg(description = "Include examples")]
        with_examples: Option<bool>,
    ) -> ToolResult {
        let spec = generate_spec_from_intent(&intent, domain, with_examples.unwrap_or(true))?;
        
        Ok(ToolResult::success(spec.to_json()?))
    }
}

struct CompleteHole;

#[tool(
    name = "complete_hole",
    description = "Generate type-compatible completions for a typed hole"
)]
impl ToolHandler for CompleteHole {
    async fn call(
        &self,
        #[arg(description = "File path containing the hole")]
        file: String,
        #[arg(description = "Hole identifier")]
        hole_id: String,
        #[arg(description = "Maximum suggestions")]
        max_suggestions: Option<u32>,
    ) -> ToolResult {
        let completions = analyze_and_complete_hole(
            &file, 
            &hole_id, 
            max_suggestions.unwrap_or(5)
        )?;
        
        Ok(ToolResult::success(completions.to_json()?))
    }
}
```

### LSP Hover with Peek

```rust
// crates/topos-lsp/src/handlers/hover.rs

use tower_lsp_server::lsp_types::*;
use facet_reflect::Peek;
use facet_pretty::PrettyPrint;

pub async fn handle_hover(
    db: &impl ToposDatabase,
    params: HoverParams,
) -> Option<Hover> {
    let file = db.file_from_uri(&params.text_document_position_params.text_document.uri)?;
    let pos = params.text_document_position_params.position;
    
    let ast = db.ast(file);
    let node = find_node_at_position(&ast, pos)?;
    
    let contents = match node {
        AstNode::TypedHole(hole) => {
            // Use Peek to inspect the hole structure
            let peek = Peek::new(&hole);
            let mut doc = String::new();
            
            doc.push_str("## Typed Hole\n\n");
            
            if let Some(name) = hole.name.as_ref() {
                doc.push_str(&format!("**Name**: `{}`\n\n", name));
            }
            
            if let Some(sig) = hole.signature.as_ref() {
                doc.push_str("**Signature**: ");
                if let Some(input) = sig.input.as_ref() {
                    doc.push_str(&format!("`{}` → ", input.pretty_print()));
                }
                if let Some(output) = sig.output.as_ref() {
                    doc.push_str(&format!("`{}`", output.pretty_print()));
                }
                doc.push_str("\n\n");
            }
            
            if !hole.constraints.is_empty() {
                doc.push_str("**Constraints**:\n");
                for c in &hole.constraints {
                    doc.push_str(&format!("- {}\n", c.pretty_print()));
                }
            }
            
            if !hole.involving.is_empty() {
                doc.push_str("\n**Involving**: ");
                let names: Vec<_> = hole.involving.iter()
                    .filter_map(|id| db.resolve_symbol(*id))
                    .map(|s| format!("`{}`", s.name))
                    .collect();
                doc.push_str(&names.join(", "));
            }
            
            doc
        }
        
        AstNode::Concept(concept) => {
            let mut doc = String::new();
            doc.push_str(&format!("## Concept `{}`\n\n", concept.name));
            
            if let Some(d) = concept.doc.as_ref() {
                doc.push_str(&format!("{}\n\n", d));
            }
            
            if !concept.fields.is_empty() {
                doc.push_str("**Fields**:\n");
                for f in &concept.fields {
                    doc.push_str(&format!("- `{}`: {}\n", f.name, f.type_expr.pretty_print()));
                }
            }
            
            doc
        }
        
        AstNode::Reference(ref_) => {
            if let Some(def) = db.resolve_reference(file, ref_.clone()) {
                format_definition_hover(&def)
            } else {
                format!("Unresolved reference: `{}`", ref_.name)
            }
        }
        
        _ => return None,
    };
    
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: contents,
        }),
        range: Some(node.span().to_lsp_range()),
    })
}
```

---

## Dependencies (Cargo.toml)

```toml
[workspace]
resolver = "2"
members = [
    "crates/topos-syntax",
    "crates/topos-analysis",
    "crates/topos-diff",
    "crates/topos-lsp",
    "crates/topos-mcp",
    "crates/topos-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.80"
license = "MIT"
repository = "https://github.com/your-org/topos"

[workspace.dependencies]
# === Reflection & Serialization (facet.rs) ===
facet = "0.28"
facet-reflect = "0.28"
facet-json = "0.28"
facet-yaml = "0.28"
facet-toml = "0.28"
facet-pretty = "0.28"
facet-assert = "0.28"

# === Incremental Computation ===
salsa = "0.18"

# === Parsing ===
tree-sitter = "0.25"
tree-sitter-topos = { path = "tree-sitter-topos" }

# === LSP ===
tower-lsp-server = "0.1"
lsp-types = "0.97"

# === MCP ===
rmcp = { version = "0.8", features = ["server"] }

# === Text Handling ===
ropey = "1.6"
codespan-reporting = "0.11"

# === Async ===
tokio = { version = "1.40", features = ["full"] }

# === CLI ===
clap = { version = "4.5", features = ["derive"] }
colored = "2.1"

# === Testing ===
insta = { version = "1.40", features = ["json"] }
proptest = "1.5"

# === Utilities ===
thiserror = "2.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Testing Strategy

### Property-Based Testing with facet

```rust
#[cfg(test)]
mod tests {
    use facet_assert::assert_same;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_serialization(concept in arb_concept()) {
            let json = facet_json::to_string(&concept).unwrap();
            let parsed: Concept = facet_json::from_str(&json).unwrap();
            assert_same!(concept, parsed);
        }
        
        #[test]
        fn parse_format_roundtrip(source in arb_topos_source()) {
            let ast = parse(&source).unwrap();
            let formatted = format_ast(&ast);
            let reparsed = parse(&formatted).unwrap();
            assert_same!(ast, reparsed);
        }
    }
}
```

### Snapshot Testing

```rust
#[test]
fn hover_on_typed_hole() {
    let source = r#"
        Behavior process:
          ensures: [?handler : `Input` -> `Output`]
    "#;
    
    let hover = get_hover(source, Position::new(2, 20));
    insta::assert_json_snapshot!(hover);
}
```

---

## Performance Considerations

1. **Salsa durability**: Standard library specs marked HIGH durability, user files LOW
2. **Lazy parsing**: Only parse files when needed for a query
3. **Incremental diffing**: facet-diff compares structurally, not byte-by-byte
4. **Parallel analysis**: Workspace diagnostics computed in parallel per file
5. **Early cutoff**: Changed whitespace doesn't invalidate semantic analysis

---

## Security Considerations & Threat Model

As an AI-bridge toolchain, Topos operates at a critical trust boundary. MCP tool exposure is exactly where prompt injection and exfiltration risks appear. This section provides a comprehensive threat model.

### Threat Model

#### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────────────┐
│                         TRUSTED ZONE                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐             │
│  │ Spec Files  │    │ Config      │    │ User Input  │             │
│  │ (.tps)      │    │ (.toml)     │    │ (CLI args)  │             │
│  └──────┬──────┘    └──────┬──────┘    └──────┬──────┘             │
│         │                  │                  │                     │
│         ▼                  ▼                  ▼                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     TOPOS CORE                                │  │
│  │  • Parser (tree-sitter)                                       │  │
│  │  • Analyzer (Salsa)                                           │  │
│  │  • Context Compiler                                           │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                              │                                      │
└──────────────────────────────┼──────────────────────────────────────┘
                               │
        ═══════════════════════╪═══════════════════════════════════════
                    TRUST BOUNDARY (MCP Protocol)
        ═══════════════════════╪═══════════════════════════════════════
                               │
┌──────────────────────────────┼──────────────────────────────────────┐
│                         UNTRUSTED ZONE                              │
│                              ▼                                      │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                     AI AGENT                                  │  │
│  │  • Claude, GPT, Cursor, Windsurf, etc.                       │  │
│  │  • May be compromised via prompt injection                    │  │
│  │  • May attempt exfiltration                                   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

#### Threat Categories

| Threat | Risk | Mitigation |
|--------|------|------------|
| **Prompt Injection** | AI agent sends malicious tool calls | Input validation, allowlists |
| **Path Traversal** | Access files outside workspace | Strict path canonicalization |
| **Data Exfiltration** | Sensitive data leaked via tool responses | Redaction, response filtering |
| **Denial of Service** | Resource exhaustion via large specs | Rate limiting, size limits |
| **Code Injection** | Malicious code in generated output | No eval, sandboxed generation |

### Security Controls

#### 1. MCP Tool Sandboxing

```rust
// crates/topos-mcp/src/sandbox.rs

pub struct McpSandbox {
    /// Allowed paths (canonicalized)
    allowed_paths: Vec<PathBuf>,
    
    /// Allowed operations
    allowed_operations: HashSet<Operation>,
    
    /// Maximum response size
    max_response_bytes: usize,
    
    /// Request rate limiter
    rate_limiter: RateLimiter,
}

impl McpSandbox {
    pub fn default_restrictive() -> Self {
        Self {
            allowed_paths: vec![],  // Must be explicitly added
            allowed_operations: hashset![
                Operation::ReadSpec,
                Operation::ValidateSpec,
                Operation::CompileContext,
            ],
            max_response_bytes: 100_000,  // 100KB
            rate_limiter: RateLimiter::new(100, Duration::from_secs(60)),
        }
    }
    
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf, SecurityError> {
        let canonical = path.canonicalize()
            .map_err(|_| SecurityError::PathTraversal)?;
        
        if !self.allowed_paths.iter().any(|p| canonical.starts_with(p)) {
            return Err(SecurityError::PathNotAllowed(canonical));
        }
        
        Ok(canonical)
    }
}
```

#### 2. Sensitive Field Redaction

```rust
// All sensitive fields marked with #[facet(sensitive)]
#[derive(Facet)]
pub struct Config {
    pub workspace_path: PathBuf,
    
    #[facet(sensitive)]  // Never logged or serialized to AI
    pub api_keys: HashMap<String, String>,
    
    #[facet(sensitive)]
    pub auth_tokens: Vec<String>,
}

// Redaction in MCP responses
pub fn prepare_mcp_response<T: Facet>(value: &T) -> String {
    facet_json::to_string_redacted(value)
}
```

#### 3. Input Validation

```rust
// All MCP tool inputs validated
#[tool(name = "read_spec")]
async fn read_spec(
    #[arg(description = "Path to spec file")]
    path: String,
    sandbox: &McpSandbox,
) -> Result<SpecContent, ToolError> {
    // 1. Validate path format
    if path.contains("..") || path.contains('\0') {
        return Err(ToolError::InvalidInput("Invalid path characters"));
    }
    
    // 2. Validate against sandbox
    let safe_path = sandbox.validate_path(Path::new(&path))?;
    
    // 3. Validate file type
    if !safe_path.extension().map_or(false, |e| e == "tps" || e == "topos") {
        return Err(ToolError::InvalidInput("Not a Topos file"));
    }
    
    // 4. Read with size limit
    let content = read_with_limit(&safe_path, sandbox.max_response_bytes)?;
    
    Ok(SpecContent { path: safe_path, content })
}
```

#### 4. Safe Defaults

```toml
# .topos/security.toml

[mcp]
# Default: deny all filesystem access
filesystem_access = "deny"

# Explicit allowlist required
allowed_paths = []

# Operations enabled by default
allowed_operations = ["read_spec", "validate_spec", "compile_context"]

# Operations disabled by default (require explicit opt-in)
disabled_operations = ["generate_code", "extract_spec", "write_file"]

[redaction]
# Always redact these patterns
patterns = [
    "password",
    "secret", 
    "api_key",
    "token",
    "credential",
]

# Never include these file patterns in responses
excluded_files = [
    ".env",
    "*.pem",
    "*.key",
    "credentials.*",
]

[limits]
max_spec_size_bytes = 1_000_000
max_response_bytes = 100_000
max_files_per_request = 10
rate_limit_per_minute = 100
```

#### 5. Offline Mode

```rust
// For environments that cannot send data to remote models
pub struct OfflineMode {
    /// Disable all MCP tools
    pub disable_mcp: bool,
    
    /// Disable telemetry
    pub disable_telemetry: bool,
    
    /// Local-only operations
    pub local_only: bool,
}

impl OfflineMode {
    pub fn from_env() -> Self {
        Self {
            disable_mcp: env::var("TOPOS_OFFLINE").is_ok(),
            disable_telemetry: env::var("TOPOS_NO_TELEMETRY").is_ok(),
            local_only: env::var("TOPOS_LOCAL_ONLY").is_ok(),
        }
    }
}
```

### Logging and Audit

```rust
// Security-relevant events logged with context
#[derive(Debug, Facet)]
pub struct SecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: SecurityEventType,
    pub source: EventSource,
    #[facet(sensitive)]
    pub details: String,
    pub outcome: EventOutcome,
}

pub enum SecurityEventType {
    PathAccessAttempt,
    RateLimitHit,
    SensitiveFieldAccess,
    McpToolInvocation,
    ValidationFailure,
}
```

---

## Round-Trip Anchors (Future)

Bidirectional sync (code↔spec) is a hard problem. The classic round-trip engineering challenge is that inverse transformations are often partial and non-injective—you cannot reliably reconstruct the "model" from the "artifact" without losing meaning or inventing it.

**V1 Strategy**: One-way sync (spec→code) with drift detection. No reverse extraction.

**V2+ Strategy**: Anchored reverse extraction with explicit markers in code.

### Anchor Format

```rust
// Code annotation format for reverse sync
// @topos(req="REQ-17", concept="Order")
pub struct Order {
    // @topos(field="id")
    pub id: Uuid,
    
    // @topos(field="status", enum="OrderStatus")  
    pub status: OrderStatus,
}

// @topos(behavior="create_order", implements="REQ-17")
pub fn create_order(items: Vec<Item>) -> Result<Order, CreateError> {
    // ...
}
```

### Anchor Requirements

1. **Stable IDs**: Every spec element has a stable identifier (`REQ-17`, `Concept:Order`, `Behavior:create_order`)
2. **Bidirectional Links**: Anchors in code reference spec IDs; spec tracks anchor locations
3. **Change Detection**: Anchored code regions tracked for modification
4. **Conflict Resolution**: Policy for when code and spec disagree (spec wins, code wins, or merge)

### Reverse Sync Constraints

- Only extract from explicitly anchored regions
- Single language at a time (Rust first, then TypeScript)
- Require explicit opt-in per file/region
- Surface ambiguities as typed holes, not guesses

---

## V2 Architecture Extensions

The following architectural extensions are planned for V2. They are documented here to inform V1 design decisions and ensure forward compatibility.

### Extension: Polyglot Analysis Layer

V1 treats foreign blocks (TypeSpec, CUE) as opaque. V2 adds shallow indexing to enable cross-block references.

```
┌─────────────────────────────────────────────────────────────────────┐
│                     topos-analysis (V2)                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐  ┌──────────────────────────────────────────────┐│
│  │ tree-sitter  │  │         Foreign Block Indexers              ││
│  │   (Topos)    │  │                                              ││
│  │   Grammar    │  │  ┌────────────┐  ┌────────────┐  ┌────────┐ ││
│  └──────┬───────┘  │  │ TypeSpec   │  │    CUE     │  │ Proto  │ ││
│         │          │  │  Parser    │  │   Parser   │  │ Parser │ ││
│         │          │  └─────┬──────┘  └─────┬──────┘  └────┬───┘ ││
│         │          └────────┼───────────────┼──────────────┼─────┘│
│         │                   │               │              │      │
│         ▼                   ▼               ▼              ▼      │
│  ┌──────────────────────────────────────────────────────────────┐ │
│  │                  Unified Symbol Table                        │ │
│  │                                                              │ │
│  │  Namespace    │ Symbol     │ Kind    │ Source               │ │
│  │  ────────────────────────────────────────────────────────── │ │
│  │  topos        │ User       │ Concept │ users.tps:45         │ │
│  │  typespec     │ User       │ Model   │ users.tps:67 (embed) │ │
│  │  cue          │ #Order     │ Schema  │ orders.tps:23 (embed)│ │
│  └──────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  Reference Resolution (V2):                                         │
│    `User` → topos.User (local definition preferred)                │
│    `typespec.User` → TypeSpec model from embedded block            │
│    `cue.#Order` → CUE schema from embedded block                   │
└─────────────────────────────────────────────────────────────────────┘
```

**V1 Compatibility**: V1 code should not depend on foreign symbols resolving. Use explicit aliasing:

```topos
# V1-compatible pattern
Concept User:
  # Mirrors the TypeSpec model below
  field id (`String`)
  field name (`String`)

```typespec
model User { id: string; name: string; }
```
```

### Extension: Evidence Automation

V2 adds the `topos-evidence` crate for automatic evidence gathering.

```
┌─────────────────────────────────────────────────────────────────────┐
│                      topos-evidence (V2)                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────┐  │
│  │   Git        │  │   Coverage   │  │   CI/CD                  │  │
│  │   Provider   │  │   Provider   │  │   Provider               │  │
│  │              │  │              │  │                          │  │
│  │  • libgit2   │  │  • LCOV      │  │  • GitHub Actions        │  │
│  │  • Commit    │  │  • Cobertura │  │  • GitLab CI             │  │
│  │  • Blame     │  │  • Istanbul  │  │  • Jenkins               │  │
│  └──────┬───────┘  └──────┬───────┘  └────────────┬─────────────┘  │
│         │                 │                       │                │
│         ▼                 ▼                       ▼                │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Evidence Aggregator                        │  │
│  │                                                              │  │
│  │  Task       │ Commit  │ PR      │ Coverage │ Pipeline        │  │
│  │  ───────────────────────────────────────────────────────── │  │
│  │  TASK-1     │ a1b2c3d │ #42     │ 94%      │ ✓ passed        │  │
│  │  TASK-2     │ f4e5d6c │ #45     │ 87%      │ ✓ passed        │  │
│  └──────────────────────────────────────────────────────────────┘  │
│         │                                                          │
│         ▼                                                          │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Spec Updater                               │  │
│  │                                                              │  │
│  │  Apply evidence updates to .tps files                        │  │
│  │  Preserve formatting, update only evidence blocks            │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

**Crate Dependencies** (V2):

```toml
[dependencies]
git2 = "0.18"                    # Git operations
octocrab = "0.32"                # GitHub API
lcov = "0.8"                     # LCOV parsing
cobertura-parser = "0.1"         # Cobertura XML
```

### Extension: Semantic Drift Engine

V2 adds LLM-based semantic comparison for prose constraints.

```
┌─────────────────────────────────────────────────────────────────────┐
│                      topos-diff (V2)                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                   Comparison Router                          │  │
│  │                                                              │  │
│  │  match spec_item.kind():                                     │  │
│  │    Concept     → structural_diff()                          │  │
│  │    Behavior    → if has_prose() { hybrid_diff() }           │  │
│  │                  else { structural_diff() }                 │  │
│  │    Invariant   → semantic_diff()                            │  │
│  └──────────────────────────────────────────────────────────────┘  │
│         │                              │                           │
│         ▼                              ▼                           │
│  ┌──────────────┐              ┌──────────────────────────────┐   │
│  │  Structural  │              │         Semantic             │   │
│  │    Diff      │              │           Diff               │   │
│  │              │              │                              │   │
│  │  facet-diff  │              │  ┌────────────────────────┐ │   │
│  │  (V1)        │              │  │   MCP Client           │ │   │
│  │              │              │  │                        │ │   │
│  │  Fast, det-  │              │  │  → analyze_spec tool   │ │   │
│  │  erministic  │              │  │  → LLM judgment        │ │   │
│  └──────────────┘              │  │  → confidence score    │ │   │
│                                │  └────────────────────────┘ │   │
│                                │                              │   │
│                                │  Slow, probabilistic,        │   │
│                                │  requires MCP connection     │   │
│                                └──────────────────────────────┘   │
│                                                                     │
│  Output: DriftReport                                               │
│    structural_issues: Vec<StructuralDrift>                         │
│    semantic_issues: Vec<SemanticDrift>  // V2 only                │
│    inconclusive: Vec<Inconclusive>      // Low confidence         │
└─────────────────────────────────────────────────────────────────────┘
```

**V1 Forward Compatibility**: The `DriftReport` type in V1 should reserve space for semantic issues:

```rust
#[derive(Facet)]
pub struct DriftReport {
    pub structural_issues: Vec<StructuralDrift>,
    
    // Reserved for V2 - always empty in V1
    #[facet(skip_if = "Vec::is_empty")]
    pub semantic_issues: Vec<SemanticDrift>,
    
    // Reserved for V2 - always empty in V1
    #[facet(skip_if = "Vec::is_empty")]
    pub inconclusive: Vec<Inconclusive>,
}
```

---

*This architecture enables Topos to be a fast, ergonomic, and AI-native specification tool while maintaining correctness through Rust's type system and facet's reflection capabilities. Security is a first-class concern given the AI-bridge nature of the toolchain.*
