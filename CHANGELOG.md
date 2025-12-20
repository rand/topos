# Changelog

All notable changes to the Topos specification are documented in this file.

## [3.1.0] - December 2025

### 🔧 Review-Driven Refinements

This release incorporates feedback from external AI reviewers, tightening scope consistency and improving adoption positioning.

### Changed

#### Traceability Reframed
- **"Every line traces"** → **"Boundary-level traceability"**: Requirements trace to externally observable behaviors and tests, not internal implementation details
- Added orphan thresholds for acceptable untraced internal code

#### Context Compiler Updated for Current Tools
- **Cursor**: Now generates `.cursor/rules/*.mdc` (MDC format with frontmatter)
- **Windsurf**: Now generates `.windsurf/rules/*.md`  
- **Cline**: Now generates `.clinerules/*.md`
- Legacy single-file formats available via `--format *-legacy`
- Added multi-tool conflict strategy configuration

#### Scope Clarified
- **V1 Beta (12 weeks)**: CLI + minimal LSP + context compiler
- **V1 Full (16 weeks)**: Production-grade with traceability and drift
- **`extract_spec` explicitly moved to V2**: Requires anchors, not V1 material
- Added explicit V1 Validation Ladder (what `topos check` actually validates)

### Added

#### Soft Constraint Guardrails
- **Soft-to-hard ratio lint**: Warn when >30% of constraints are `[~]`
- **Hardening task association**: Link `[~]` to tasks for eventual precision
- **`[~permanent]` marker**: Explicitly mark intentionally-soft-forever constraints
- **Soft constraint report**: `topos trace --soft-constraints`

#### Stable ID Format Recommendations
- Semantic prefixes: `REQ-AUTH-1` over `REQ-1`
- Zero-padding for sorting: `REQ-001`
- `topos rename` command for safe ID refactoring

#### Maintenance Model
- Ownership model: Who updates what, when
- PR workflow integration with CI examples
- Drift alert triage guidance
- Recommended ceremonies (per-PR, weekly, sprint)

#### Risk Register
- Documented known risks with likelihood, impact, and mitigations
- Covers: adoption friction, spec rot, soft constraint sprawl, security, multi-language complexity

#### V2 Feature Documentation
Comprehensive design notes for deferred features to enable future implementation:

- **Polyglot Symbol Resolution**: Shallow tree-sitter indexing of TypeSpec/CUE blocks, unified symbol table design, implementation sketches
- **Auto-Evidence Gathering (`topos gather`)**: Git/GitHub/coverage integration, CI workflow examples, provider architecture
- **Semantic Drift Detection**: LLM-as-Judge via MCP, comparison strategy routing, confidence thresholds, hybrid structural+semantic approach

### Fixed

- Phase 4 success criteria no longer claims bidirectional sync (V1 is one-way)
- CommonMark compatibility statement strengthened: "Topos IS Markdown"

---

## [3.0.0] - December 2025

### 🎯 De-Risked Roadmap

This release represents a significant course correction based on external review feedback. The key insight: bidirectional sync is a tarpit that can consume unlimited schedule. V1 now focuses on proven value with clear scope.

### Changed

#### Scope Reduction
- **Bidirectional sync deferred to V2**: V1 is one-way (spec→code) with drift detection only
- **Reverse extraction requires anchors**: Future code→spec flow needs explicit `@topos()` annotations
- **Single-language first**: V2 reverse extraction targets Rust only, not multiple languages

#### Markdown Compatibility (Adoption Multiplier)
- **CommonMark-compatible syntax**: Topos files render correctly in any Markdown viewer
- **Graceful degradation**: Specs are readable documentation even without Topos tooling
- **PR-friendly**: Standard diff tools work without special support
- **Gradual adoption path**: Start with Markdown, add structure incrementally

### Added

#### Context Compiler (New Feature)
- **`topos context TASK-N`**: Generate focused AI context for specific tasks
- **Multiple output formats**: `.cursorrules`, `.clinerules`, `.windsurfrules`, JSON, Markdown
- **Intelligent pruning**: Only include requirements, concepts, behaviors relevant to the task
- **MCP tool**: `compile_context` exposed for AI agents to request focused context

#### Aesthetic Blocks (New Feature)
- **`Aesthetic` keyword**: Capture non-functional, subjective, "vibe" requirements
- **Soft constraints `[~]`**: Mark approximate or aesthetic requirements
- **Context integration**: Aesthetics included in context compilation for UI tasks

#### Evidence-Based Tasks (New Feature)
- **`evidence:` block**: Required proof of task completion
- **Supported evidence types**: `pr:`, `commit:`, `coverage:`, `benchmark:`, `review:`
- **Traceability score**: CLI can compute measurable traceability, not just vibes

#### Foreign Blocks (TypeSpec/CUE Integration)
- **Embedded TypeSpec**: Use TypeSpec for API schemas within Topos specs
- **Embedded CUE**: Use CUE for constraint validation within Topos specs
- **Parasitic positioning**: Topos is the spine; best-in-class tools for specialized domains

#### Security & Threat Model
- **Comprehensive threat model**: Trust boundaries, threat categories, mitigations
- **MCP sandboxing**: Default-deny filesystem access, explicit allowlists
- **Sensitive field redaction**: `#[facet(sensitive)]` for automatic redaction in logs/responses
- **Offline mode**: `TOPOS_OFFLINE=1` for environments that can't send data to remote models

### Changed

#### Realistic Timeline
- **16 weeks for V1** (was 20 weeks for everything)
- **Clear phase boundaries**: Foundation → Parser → Analysis → LSP → CLI/MCP → Polish
- **Exit criteria per phase**: Not just "implement X" but "X works for Y scenarios"

#### Success Metrics
- **User value metrics primary**: Time-to-skeleton, drift precision/recall, context usefulness
- **Technical metrics secondary**: Parse time, memory usage, LSP latency

#### Documentation
- **README rewritten**: Clear "What Topos Is / Isn't" section
- **CONTEXT_COMPILER.md**: Complete documentation for new feature
- **ARCHITECTURE.md**: Added threat model and round-trip anchor specification

### Removed

- **Implicit bidirectional sync promise**: Now explicitly V2+ with anchors required
- **Over-optimistic timeline**: Replaced with phased, de-risked plan

### Why These Changes?

Based on external review, the original proposal had three critical risks:

1. **Scope/schedule mismatch**: 20 weeks for parser + LSP + bidirectional sync + multi-language extraction was "spirited" at best
2. **Bidirectional sync underspecified**: The hard part isn't diffing structs—it's stable IDs, provenance, and conflict resolution
3. **Adoption friction**: A new DSL without Markdown compatibility faces the "yet another file format" problem

This release addresses all three:
- V1 scope is achievable in 16 weeks
- Bidirectional sync is explicitly deferred with anchor requirements documented
- Markdown compatibility makes adoption friction near-zero

---

## [2.0.0] - December 2025

### 🚀 Major Technology Uplevel

This release represents a comprehensive technology refresh, adopting the latest Rust ecosystem tools as of December 2025.

### Added

#### facet.rs Integration (Core Change)
- **Single derive macro**: All AST types now use `#[derive(Facet)]` instead of multiple traits
- **Peek API**: Used for LSP hover and debugging - read-only inspection of values
- **Partial API**: Used during parsing for incremental AST construction
- **facet-diff**: Native structural diffing for spec↔code drift detection
- **facet-pretty**: Colored, redacted output for diagnostics and reports
- **facet-json/yaml/toml**: Unified serialization replacing serde ecosystem

#### Modern MCP Integration
- **rmcp 0.8+**: Official Rust MCP SDK from modelcontextprotocol organization
- **MCP 2025-06-18 spec**: Latest protocol specification support
- **Six core tools**: create_spec, generate_code, extract_spec, complete_hole, analyze_spec, trace_requirement
- **Macro-based tool definition**: Clean `#[tool(...)]` attribute for declaring MCP tools

#### LSP Server Updates
- **tower-lsp-server 0.1+**: Community fork with active maintenance
- **lsp-types 0.97**: Latest LSP type definitions
- **Native async traits**: Removed `#[async_trait]` dependency for cleaner implementations

#### Bidirectional Sync Engine
- **Spec→Code drift detection**: Using facet-diff for structural comparison
- **Code→Spec extraction**: Generate specs from existing codebases
- **Conflict resolution**: Identify and suggest resolutions for diverged specs/code
- **Sync status tracking**: InSync, SpecAhead, CodeAhead, Diverged states

### Changed

#### Dependency Updates
| Dependency | Old Version | New Version | Notes |
|------------|-------------|-------------|-------|
| salsa | 0.17 | 0.18+ | Better durability, parallel eval support |
| tree-sitter | 0.22 | 0.25 | Performance improvements |
| tower-lsp | 0.20 | tower-lsp-server 0.1 | Community fork |
| lsp-types | 0.94 | 0.97 | Latest LSP spec |
| tokio | 1.36 | 1.40 | Latest async runtime |
| clap | 4.4 | 4.5 | Latest CLI parsing |
| N/A | serde 1.0 | facet 0.28 | **Replaced serde with facet** |
| N/A | N/A | rmcp 0.8 | **New: Official MCP SDK** |
| N/A | N/A | facet-diff | **New: Structural diffing** |

#### Architecture Changes
- **Unified type system**: All AST nodes derive `Facet` instead of `Serialize`, `Deserialize`, `Debug`, `PartialEq`
- **Fast compilation**: facet uses `unsynn` instead of `syn`, dramatically reducing build times
- **Runtime reflection**: Can inspect and manipulate types at runtime without monomorphization
- **Structural comparison**: `facet-diff` enables comparing values without `PartialEq`

#### CLI Changes
- New `drift` command for spec↔code drift detection
- Updated `mcp` command to use rmcp 0.8
- Improved output formatting with facet-pretty

### Removed

- **serde dependency**: Replaced entirely by facet.rs
- **Manual PartialEq implementations**: facet-assert handles structural comparison
- **Custom Debug implementations**: facet-pretty provides better output

### Technical Details

#### Why facet.rs?

1. **Single derive macro** - One `#[derive(Facet)]` replaces 5+ trait derives
2. **Fast compilation** - Uses unsynn (not syn), compiles much faster
3. **Runtime reflection** - Peek allows inspecting any Facet type at runtime
4. **Structural diffing** - facet-diff enables spec↔code drift detection
5. **Pretty printing** - facet-pretty provides colored, redacted output
6. **Serialization** - facet-json/yaml/toml for all formats

#### Why rmcp over other MCP implementations?

1. **Official SDK** - Maintained by modelcontextprotocol organization
2. **Latest spec** - Supports MCP 2025-06-18 specification
3. **Tokio native** - Clean async/await integration
4. **Macro-based tools** - `#[tool(...)]` for clean tool definitions
5. **Active development** - Regular updates and fixes

#### Why tower-lsp-server over tower-lsp?

1. **Active maintenance** - Community fork with regular updates
2. **Latest lsp-types** - Uses 0.97 (tower-lsp stuck on 0.94)
3. **No async_trait** - Cleaner trait implementations
4. **Better error handling** - Uses fluent_uri

### Migration Guide

#### From serde to facet

```rust
// Before (serde)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Concept {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}

// After (facet)
#[derive(Debug, Clone, Facet)]
pub struct Concept {
    pub name: String,
    #[facet(skip_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}
```

#### From custom diff to facet-diff

```rust
// Before (manual comparison)
impl Concept {
    fn diff(&self, other: &Concept) -> Vec<Change> {
        // ... manual field-by-field comparison
    }
}

// After (facet-diff)
use facet_reflect::check_same_report;
let result = check_same_report(&spec_model, &code_model);
```

#### From tower-lsp to tower-lsp-server

```rust
// Before
use tower_lsp::{LanguageServer, LspService, Server};
#[tower_lsp::async_trait]
impl LanguageServer for Backend { ... }

// After
use tower_lsp_server::{LanguageServer, LspService, Server};
impl LanguageServer for Backend { ... }  // No async_trait needed
```

### Performance Targets

| Operation | Target | Technology |
|-----------|--------|------------|
| Parse (1000 lines) | < 50ms | tree-sitter 0.25 |
| Incremental reparse | < 5ms | Salsa durability |
| Go-to-definition | < 30ms | Salsa memoization |
| Spec↔Code diff | < 100ms | facet-diff |
| MCP tool call | < 200ms | rmcp async |

---

## [1.0.0] - November 2025

### Added
- Initial Topos language specification
- Five-layer structure: Principles, Requirements, Design, Concepts, Tasks
- Typed holes with signatures and constraints
- EARS notation for requirements
- BDD acceptance criteria
- Multi-file project support with imports
- LSP specification
- MCP tool definitions
- Traceability system

### Notes
- This was the initial design specification
- No implementation code was included
- Technology choices were tentative

---

## Version History

| Version | Date | Focus |
|---------|------|-------|
| 3.1.0 | Dec 2025 | Review-driven refinements, boundary traceability, soft constraint guardrails |
| 3.0.0 | Dec 2025 | De-risked roadmap, context compiler, aesthetics, security |
| 2.0.0 | Dec 2025 | facet.rs + modern tooling uplevel |
| 1.0.0 | Nov 2025 | Initial specification |
