# Topos Implementation Execution Plan

**Version**: 3.0.0  
**Date**: December 2025  
**Status**: Implementation Spec  
**Target**: Production-grade toolchain with de-risked, phased delivery

---

## Executive Summary

This document provides a **de-risked execution plan** for implementing the Topos toolchain. The plan explicitly acknowledges that bidirectional sync is hard and defers it to V2, focusing V1 on the core value proposition: spec→code flow with traceability and drift detection.

### Key Changes from v2.0

1. **Scope reduction**: V1 is one-way (spec→code), no reverse extraction
2. **Realistic timeline**: 16 weeks for V1 (not 20 weeks for everything)
3. **Evidence-based milestones**: Success metrics based on user value, not just technical correctness
4. **Explicit V1/V2/V3 boundaries**: Clear scope for each phase

### Key Technology Choices (December 2025)

| Component | Library | Version | Why |
|-----------|---------|---------|-----|
| Reflection | facet.rs | 0.28+ | Single derive, fast compile, structural diffing |
| Incremental | Salsa | 0.18+ | Proven in rust-analyzer, durability optimization |
| Parsing | tree-sitter | 0.25+ | Sub-ms incremental parsing, error recovery |
| LSP | tower-lsp-server | 0.1+ | Community fork with lsp-types 0.97 |
| MCP | rmcp | 0.8+ | Official Rust SDK, MCP 2025-06-18 spec |
| Async | tokio | 1.40+ | Standard async runtime |
| CLI | clap | 4.5+ | Derive-based argument parsing |

---

## Version Boundaries

### V1 Beta: Credible First Release (12 weeks)
**Goal**: Working CLI + minimal LSP + context compiler

- Parse and validate `.tps` files
- CLI: `check`, `format`, `trace`, `context`
- Minimal LSP: diagnostics, go-to-definition, hover
- Context compiler output for Cursor, Windsurf, Cline (current formats)
- Traceability graph export (JSON, Markdown)

**Exit criteria**: A developer can write a spec, generate context, and use it in their AI IDE.

### V1: Full Release (16 weeks)
**Goal**: Production-grade toolchain with complete traceability

Everything in V1 Beta, plus:
- LSP: find-references, completions, code actions
- CLI: `drift` (one-way, structural only)
- MCP tools: `validate_spec`, `summarize_spec`, `compile_context`
- Traceability reports (REQ→Behavior→Task→File)
- VS Code extension (syntax + LSP client)
- Evidence validation (links exist, files exist)

**NOT in V1** (explicitly deferred):
- `extract_spec` MCP tool (requires anchors, V2)
- Reverse extraction (code→spec)
- Bidirectional sync
- Multi-language code extraction
- Typed hole suggestions via LLM
- Foreign block semantic validation (TypeSpec/CUE parsing)

### V2: Anchored Reverse Flow (Future)
**Goal**: Code→spec extraction with explicit anchors

- Anchor annotation format (`@topos(req="REQ-N")`)
- `extract_spec` MCP tool (anchored regions only)
- Single-language reverse extraction (Rust first)
- Anchored region tracking
- Spec update suggestions from code changes
- **Polyglot Symbol Resolution** (see below)
- **Auto-Evidence Gathering** (see below)
- **Semantic Drift Detection** (see below)

### V3: Research (Future)
**Goal**: True bidirectional sync

- Multi-language extraction
- Conflict resolution policies
- Constraint solver integration
- Formal verification pathway

---

## Deferred Features: V2+ Design Notes

The following features were identified during V1 design but explicitly deferred. This section provides implementation guidance for future phases.

### Feature: Polyglot Symbol Resolution ("Glass Box" Foreign Blocks)

**Problem Statement**

V1 treats foreign blocks (TypeSpec, CUE) as opaque text blobs. If a Topos `Behavior` references `User` and `User` is defined in an embedded TypeSpec block, the semantic graph is broken—Topos can't "see inside" the foreign block.

```topos
# This works in V1 (opaque aliasing)
Concept User:
  see: typespec block below

```typespec
model User {
  id: string;
  @minLength(1) name: string;
}
```

# This DOESN'T work in V1 (cross-block reference)
Behavior create_user:
  returns: `User`  # ← Which User? Topos doesn't know about TypeSpec.User
```

**V2 Solution: Shallow Indexing via tree-sitter**

Use tree-sitter's multi-language parsing to extract top-level declarations from foreign blocks into the Topos symbol table.

```
┌──────────────────────────────────────────────────────────────────┐
│                    topos-analysis crate                          │
├──────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ tree-sitter  │  │ tree-sitter  │  │ tree-sitter  │           │
│  │   (Topos)    │  │  (TypeSpec)  │  │    (CUE)     │           │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘           │
│         │                 │                 │                    │
│         ▼                 ▼                 ▼                    │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │              Polyglot Symbol Indexer                         ││
│  │                                                              ││
│  │  • Extract model/struct/schema names                         ││
│  │  • Map to Topos namespace: `typespec.User`, `cue.Order`     ││
│  │  • Shallow only: names + field names, not full semantics    ││
│  └──────────────────────────────────────────────────────────────┘│
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │                 Unified Symbol Table                         ││
│  │                                                              ││
│  │  `User` → resolves to typespec.User (line 45, users.tps)    ││
│  │  `#Order` → resolves to cue.#Order (line 89, orders.tps)    ││
│  └──────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────┘
```

**Implementation Sketch**

```rust
// crates/topos-analysis/src/foreign.rs

use tree_sitter::{Parser, Language};

pub struct ForeignIndexer {
    typespec_parser: Parser,
    cue_parser: Parser,
}

impl ForeignIndexer {
    pub fn index_block(&self, language: &str, content: &str) -> Vec<ForeignSymbol> {
        match language {
            "typespec" => self.index_typespec(content),
            "cue" => self.index_cue(content),
            _ => vec![], // Unknown language, skip
        }
    }
    
    fn index_typespec(&self, content: &str) -> Vec<ForeignSymbol> {
        let tree = self.typespec_parser.parse(content, None).unwrap();
        let mut symbols = vec![];
        
        // Walk tree looking for model/interface/enum declarations
        for node in tree.root_node().children(&mut tree.walk()) {
            if node.kind() == "model_statement" {
                let name = node.child_by_field_name("name")
                    .map(|n| content[n.byte_range()].to_string());
                if let Some(name) = name {
                    symbols.push(ForeignSymbol {
                        name,
                        kind: SymbolKind::Model,
                        namespace: "typespec".into(),
                        fields: self.extract_typespec_fields(node, content),
                    });
                }
            }
        }
        symbols
    }
}

#[derive(Debug, Facet)]
pub struct ForeignSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub namespace: String,
    pub fields: Vec<String>, // Shallow: just field names
}
```

**Context Compiler Impact**

When compiling context for a task, include the relevant foreign block snippets:

```markdown
## Relevant Concepts

### User (from TypeSpec)
```typespec
model User {
  id: string;
  @minLength(1) name: string;
}
```
```

**Risks & Mitigations**

| Risk | Mitigation |
|------|------------|
| tree-sitter grammar availability | TypeSpec and CUE both have maintained grammars |
| Semantic mismatch (TypeSpec `model` ≠ Topos `Concept`) | Shallow indexing only—names, not semantics |
| Version drift in foreign language grammars | Pin grammar versions, document compatibility |

---

### Feature: Auto-Evidence Gathering (`topos gather`)

**Problem Statement**

V1 requires manual entry of evidence fields:

```topos
## TASK-1: Implement User model [REQ-1]
file: src/models/user.ts
evidence:
  pr: https://github.com/org/repo/pull/42
  commit: a1b2c3d
  coverage: 94%
status: done
```

This is toil. Developers will stop updating evidence within a week, and stale evidence is worse than no evidence (false confidence).

**V2 Solution: Evidence Daemon**

A CLI command that auto-populates evidence by querying Git and coverage tools:

```bash
topos gather [--path specs/]
```

**Implementation Sketch**

```rust
// crates/topos-cli/src/commands/gather.rs

use git2::Repository;
use std::path::PathBuf;

pub async fn run_gather(spec_path: PathBuf) -> Result<()> {
    let db = load_workspace(&spec_path)?;
    let repo = Repository::open_from_env()?;
    
    for task in db.tasks_with_pending_evidence() {
        let mut updates = EvidenceUpdates::default();
        
        // 1. Git Evidence
        if let Some(file_path) = &task.file {
            // Find last commit touching this file
            let commit = find_last_commit(&repo, file_path)?;
            updates.commit = Some(commit.id().to_string()[..7].to_string());
            
            // Find associated PR (requires GitHub API)
            if let Some(pr) = find_pr_for_commit(&commit).await? {
                updates.pr = Some(pr.html_url);
            }
        }
        
        // 2. Coverage Evidence
        if let Some(coverage) = lookup_coverage(&task.file, &task.tests)? {
            updates.coverage = Some(coverage);
        }
        
        // 3. Apply updates
        if !updates.is_empty() {
            db.update_task_evidence(task.id, updates)?;
            println!("  Updated evidence for {}", task.id);
        }
    }
    
    Ok(())
}

fn find_last_commit(repo: &Repository, file: &Path) -> Result<Commit> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    
    for oid in revwalk {
        let commit = repo.find_commit(oid?)?;
        // Check if commit touched this file
        if commit_touches_file(&commit, file)? {
            return Ok(commit);
        }
    }
    
    Err(anyhow!("No commits found for {}", file.display()))
}

async fn find_pr_for_commit(commit: &Commit) -> Result<Option<PullRequest>> {
    // Use GitHub API to find PR containing this commit
    // GET /repos/{owner}/{repo}/commits/{sha}/pulls
    // Requires GITHUB_TOKEN environment variable
    todo!()
}

fn lookup_coverage(file: &Option<PathBuf>, tests: &Option<PathBuf>) -> Result<Option<f32>> {
    // Parse standard coverage formats:
    // - LCOV: coverage/lcov.info
    // - Cobertura: coverage/cobertura.xml
    // - Istanbul JSON: coverage/coverage-final.json
    
    let lcov_path = Path::new("coverage/lcov.info");
    if lcov_path.exists() {
        let lcov = parse_lcov(lcov_path)?;
        if let Some(file) = file {
            return Ok(lcov.coverage_for(file));
        }
    }
    
    Ok(None)
}
```

**Integration Points**

| Source | Data | API/Format |
|--------|------|------------|
| Git | Commit hash, author, date | libgit2 / git2-rs |
| GitHub | PR URL, review status | REST API v3 |
| GitLab | MR URL, pipeline status | REST API v4 |
| LCOV | Line coverage % | Text format |
| Cobertura | Line/branch coverage | XML format |
| Istanbul | Statement/function coverage | JSON format |

**CLI UX**

```bash
# Gather evidence for all pending tasks
topos gather

# Gather for specific task
topos gather TASK-17

# Dry-run (show what would be updated)
topos gather --dry-run

# Output:
# Gathering evidence...
#   TASK-1: Updated commit (a1b2c3d), pr (#42), coverage (94%)
#   TASK-2: Updated commit (f4e5d6c), coverage (87%)
#   TASK-3: No changes (already current)
# 
# Updated 2 tasks, skipped 1
```

**CI Integration**

```yaml
# .github/workflows/evidence.yml
on:
  push:
    branches: [main]

jobs:
  gather-evidence:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install topos
      - run: topos gather --commit
      - uses: stefanzweifel/git-auto-commit-action@v5
        with:
          commit_message: "chore: update task evidence"
          file_pattern: "specs/**/*.tps"
```

**Risks & Mitigations**

| Risk | Mitigation |
|------|------------|
| GitHub API rate limits | Cache responses, batch requests |
| Coverage tool fragmentation | Support top 3 formats (LCOV, Cobertura, Istanbul) |
| Multi-repo projects | Config for repo mapping |
| Evidence freshness race conditions | Atomic updates, last-write-wins |

---

### Feature: Semantic Drift Detection (LLM-as-Judge)

**Problem Statement**

V1 drift detection is structural: it compares types, field names, and function signatures using `facet-diff`. This works well for:

```topos
Concept Order:
  field status (`OrderStatus`)  # ← Can check if code has this field
```

But it fails completely for behavioral requirements:

```topos
Behavior retry_payment:
  ensures:
    system retries up to 3 times with exponential backoff
```

You cannot structurally diff "retries up to 3 times with exponential backoff" against a Rust function. The prose is not machine-checkable without understanding intent.

**V2 Solution: LLM-as-Judge via MCP**

Use the LLM (via MCP tool) to perform semantic comparison between spec prose and code behavior.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Drift Detection Pipeline                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────────────┐   │
│  │   Spec      │     │   Code      │     │   Comparison        │   │
│  │  (Topos)    │     │  (Rust/TS)  │     │   Strategy          │   │
│  └──────┬──────┘     └──────┬──────┘     └──────────┬──────────┘   │
│         │                   │                       │               │
│         ▼                   ▼                       ▼               │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Drift Comparator                          │  │
│  │                                                              │  │
│  │  if is_structural(spec_item):                               │  │
│  │      return facet_diff(spec, code)  # Fast, deterministic   │  │
│  │  else:                                                       │  │
│  │      return llm_judge(spec, code)   # Slow, probabilistic   │  │
│  └──────────────────────────────────────────────────────────────┘  │
│         │                                                           │
│         ▼                                                           │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                    Drift Report                              │  │
│  │                                                              │  │
│  │  Structural Drift (high confidence):                        │  │
│  │    • Order.status: spec says `OrderStatus`, code has `str`  │  │
│  │                                                              │  │
│  │  Semantic Drift (medium confidence, 0.73):                  │  │
│  │    • retry_payment: spec says "3 retries", code does 5      │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

**Implementation Sketch**

```rust
// crates/topos-diff/src/semantic.rs

use rmcp::{tool, ToolHandler};

#[derive(Debug, Facet)]
pub struct SemanticDriftResult {
    pub spec_summary: String,
    pub code_summary: String,
    pub alignment_score: f32,  // 0.0 = completely misaligned, 1.0 = perfect match
    pub discrepancies: Vec<String>,
    pub confidence: f32,       // How confident is the LLM in this assessment
}

pub async fn check_semantic_drift(
    spec_behavior: &Behavior,
    code_function: &ExtractedFunction,
    mcp_client: &McpClient,
) -> Result<SemanticDriftResult> {
    let prompt = format!(r#"
You are a code reviewer checking if an implementation matches its specification.

## Specification (from Topos spec)
Name: {name}
Ensures: {ensures}
Requires: {requires}

## Implementation (from code)
```{language}
{code}
```

## Task
1. Does the implementation satisfy the specification's requirements and ensures clauses?
2. Are there any discrepancies between spec and code?
3. Rate the alignment from 0.0 (completely wrong) to 1.0 (perfect match).

Respond in JSON format:
{{
  "alignment_score": <float>,
  "discrepancies": [<string>, ...],
  "reasoning": "<explanation>"
}}
"#,
        name = spec_behavior.name,
        ensures = spec_behavior.ensures.join("\n"),
        requires = spec_behavior.requires.join("\n"),
        language = code_function.language,
        code = code_function.source,
    );
    
    let response = mcp_client.call_tool("analyze_spec", json!({
        "prompt": prompt,
        "response_format": "json"
    })).await?;
    
    let result: LlmJudgment = serde_json::from_str(&response)?;
    
    Ok(SemanticDriftResult {
        spec_summary: format!("{}: {}", spec_behavior.name, spec_behavior.ensures.first().unwrap_or(&"".to_string())),
        code_summary: code_function.signature.clone(),
        alignment_score: result.alignment_score,
        discrepancies: result.discrepancies,
        confidence: estimate_confidence(&result),
    })
}

fn estimate_confidence(result: &LlmJudgment) -> f32 {
    // Heuristics for confidence:
    // - Shorter code = higher confidence (less to misunderstand)
    // - Clear discrepancies = higher confidence
    // - Hedging language in reasoning = lower confidence
    
    let hedging_words = ["might", "possibly", "unclear", "ambiguous"];
    let hedging_count = hedging_words.iter()
        .filter(|w| result.reasoning.to_lowercase().contains(*w))
        .count();
    
    let base_confidence = 0.85;
    let hedging_penalty = hedging_count as f32 * 0.1;
    
    (base_confidence - hedging_penalty).max(0.3)
}
```

**Comparison Strategy Selection**

```rust
// crates/topos-diff/src/compare.rs

#[derive(Debug, Clone, Facet)]
pub enum ComparisonStrategy {
    /// Fast structural check using facet-diff
    /// Used for: Concepts, field types, function signatures
    Structural,
    
    /// Semantic check using LLM-as-Judge
    /// Used for: Behavior ensures/requires, prose constraints
    Semantic { confidence_threshold: f32 },
    
    /// Hybrid: structural first, semantic for prose
    Hybrid { semantic_threshold: f32 },
}

impl ComparisonStrategy {
    pub fn for_spec_item(item: &SpecItem) -> Self {
        match item {
            SpecItem::Concept(_) => Self::Structural,
            SpecItem::Behavior(b) if b.has_prose_constraints() => {
                Self::Hybrid { semantic_threshold: 0.7 }
            }
            SpecItem::Behavior(_) => Self::Structural,
            SpecItem::Invariant(_) => Self::Semantic { confidence_threshold: 0.6 },
        }
    }
}

pub async fn compare(
    spec: &SpecItem,
    code: &CodeItem,
    strategy: ComparisonStrategy,
    mcp: Option<&McpClient>,
) -> DriftResult {
    match strategy {
        ComparisonStrategy::Structural => {
            structural_diff(spec, code)
        }
        ComparisonStrategy::Semantic { confidence_threshold } => {
            let mcp = mcp.ok_or(DriftError::McpRequired)?;
            let result = semantic_diff(spec, code, mcp).await?;
            if result.confidence < confidence_threshold {
                DriftResult::Inconclusive { reason: "Low confidence".into() }
            } else {
                result.into()
            }
        }
        ComparisonStrategy::Hybrid { semantic_threshold } => {
            // Try structural first
            let structural = structural_diff(spec, code);
            if structural.has_issues() {
                return structural;
            }
            
            // Then semantic for prose parts
            if let Some(mcp) = mcp {
                if spec.has_prose_constraints() {
                    let semantic = semantic_diff(spec, code, mcp).await?;
                    if semantic.alignment_score < semantic_threshold {
                        return semantic.into();
                    }
                }
            }
            
            DriftResult::Aligned
        }
    }
}
```

**CLI UX**

```bash
# Structural drift only (fast, V1 behavior)
topos drift --structural

# Semantic drift (requires MCP connection)
topos drift --semantic

# Hybrid (default in V2)
topos drift

# Output:
# Checking drift...
# 
# ══ Structural Drift ══
# ✗ Order.status: spec expects `OrderStatus`, code has `String`
# ✗ User.email: spec expects unique constraint, code has none
# 
# ══ Semantic Drift ══
# ⚠ retry_payment (confidence: 0.78)
#   Spec: "retries up to 3 times with exponential backoff"
#   Code: Retries 5 times with linear backoff
#   Discrepancy: Retry count mismatch (3 vs 5), backoff strategy differs
# 
# ✓ process_order (confidence: 0.91)
#   Aligned
```

**Cost & Performance Considerations**

| Concern | Mitigation |
|---------|------------|
| LLM API costs | Cache results by content hash, only re-check on changes |
| Latency (1-5s per check) | Parallel checks, progress indicator, `--structural` flag |
| Non-determinism | Report confidence, require threshold, human review for low confidence |
| False positives | Conservative thresholds (0.7+), clear "inconclusive" state |

**Risks & Mitigations**

| Risk | Mitigation |
|------|------------|
| LLM hallucination | Confidence thresholds, human review for drift alerts |
| Context window limits | Chunk large functions, summarize context |
| Model drift (LLM behavior changes) | Pin model version in config, regression tests |
| Offline environments | Graceful degradation to structural-only |

---

## Timeline Overview (V1)

| Phase | Focus | Duration | Key Deliverable | Exit Criteria |
|-------|-------|----------|-----------------|---------------|
| **0** | Foundation | Week 0-1 | Project setup, decisions locked | Markdown-compat decision finalized |
| **1** | Parser | Weeks 2-5 | tree-sitter grammar + AST | Parse 100% of example specs |
| **2** | Analysis | Weeks 6-9 | Salsa DB + resolution | Diagnostics for all error types |
| **3** | LSP | Weeks 10-12 | Core LSP features | Hover, goto-def, find-refs working |
| **4** | CLI + MCP | Weeks 13-15 | CLI commands + MCP tools | All V1 commands functional |
| **5** | Polish | Week 16 | Documentation + extension | VS Code extension published |

---

## Success Metrics

### User Value Metrics (Primary)

| Metric | Target | How Measured |
|--------|--------|--------------|
| Time to first skeleton | < 5 min | From spec to generated code structure |
| Drift detection precision | > 90% | True positives / (true positives + false positives) |
| Drift detection recall | > 80% | True positives / (true positives + false negatives) |
| Context compilation usefulness | > 70% | User survey: "Did the context help?" |
| Human edit delta | < 30% | Lines changed after AI generation |

### Technical Metrics (Secondary)

| Metric | Target | How Measured |
|--------|--------|--------------|
| Initial parse (1000 lines) | < 50ms | Benchmark suite |
| Incremental reparse | < 5ms | Benchmark suite |
| Go-to-definition | < 30ms | LSP latency |
| Spec↔Code diff | < 100ms | Benchmark suite |
| Memory (10K line spec) | < 100MB | Peak RSS |

---

## V1 Validation Ladder

This is what `topos check` actually validates in V1, in order of complexity:

### Level 1: Syntax (Parser)
- [ ] Valid Topos/Markdown syntax
- [ ] All blocks properly closed
- [ ] Indentation consistent

### Level 2: Structure (Analysis)
- [ ] ID uniqueness (`REQ-1` not duplicated)
- [ ] Reference resolution (all `` `TypeName` `` references resolve)
- [ ] Section structure valid (Requirements before Tasks, etc.)

### Level 3: Anchors (File System)
- [ ] `file:` paths exist in workspace
- [ ] `tests:` paths exist in workspace
- [ ] Import paths resolve to valid `.tps` files

### Level 4: Evidence (Links)
- [ ] `pr:` URLs are well-formed
- [ ] `commit:` hashes are valid format (not verified against Git)
- [ ] `coverage:` is a valid percentage

### Level 5: Traceability (Graph)
- [ ] All Tasks link to at least one Requirement
- [ ] All Behaviors have `Implements REQ-N`
- [ ] No orphan Requirements (REQs with no implementing Behaviors)

**What V1 does NOT validate:**
- Foreign block semantics (TypeSpec/CUE syntax not checked)
- PR/commit existence (we don't call GitHub API)
- Code-spec semantic alignment (that's V2+ with LLM-as-judge)

---

## Phase 0: Foundation (Weeks 0-1)

### 0.1 Project Setup (Day 1-2)

```bash
# Initialize workspace
cargo new --lib topos
cd topos
mkdir -p crates/{topos-syntax,topos-analysis,topos-lsp,topos-mcp,topos-cli}
mkdir -p tree-sitter-topos
```

### 0.2 Critical Decisions (Day 3-5)

**Decision 1: Markdown Compatibility**

Commit to CommonMark-compatible syntax. This means:
- Section markers are standard Markdown headings
- Keywords recognized at line start (not arbitrary positions)
- Foreign blocks use fenced code block syntax
- Files render correctly without Topos tooling

**Decision 2: Stable IDs**

Define stable identifier format for future anchoring:
- `REQ-{project}-{number}` (e.g., `REQ-SHOP-17`)
- `Concept:{name}` (e.g., `Concept:Order`)
- `Behavior:{name}` (e.g., `Behavior:create_order`)
- `TASK-{number}` (e.g., `TASK-42`)

**Decision 3: Evidence Schema**

Lock the evidence field schema for Tasks:
```topos
evidence:
  pr: URL
  commit: HASH
  coverage: PERCENT
  benchmark: TEXT
```

### 0.3 Test Corpus (Day 5-7)

Create a corpus of 20+ example specs covering:
- Simple single-file specs
- Multi-file projects with imports
- All section types exercised
- Typed holes of various forms
- Aesthetic blocks
- Foreign blocks (TypeSpec, CUE)
- Edge cases (empty sections, minimal specs)

**Exit Criteria**: All decisions documented, test corpus complete.

---

## Phase 1: Parser (Weeks 2-5)

### 1.1 Tree-Sitter Grammar (Week 1)

Create the Topos grammar with external scanner for indentation.

**File**: `tree-sitter-topos/grammar.js`

```javascript
module.exports = grammar({
  name: 'topos',

  extras: $ => [/\s/, $.comment],
  
  externals: $ => [
    $._indent,
    $._dedent,
    $._newline,
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => seq(
      optional($.spec_declaration),
      repeat($.section)
    ),

    spec_declaration: $ => seq(
      'spec',
      field('name', $.identifier),
      $._newline
    ),

    comment: $ => token(seq('#', /.*/)),

    // === SECTIONS ===
    section: $ => choice(
      $.principles_section,
      $.requirements_section,
      $.design_section,
      $.concepts_section,
      $.behaviors_section,
      $.tasks_section,
    ),

    principles_section: $ => seq(
      $.heading,
      'Principles',
      $._newline,
      repeat($.principle)
    ),

    principle: $ => seq(
      '-',
      field('name', $.identifier),
      ':',
      field('description', $.prose),
      $._newline
    ),

    requirements_section: $ => seq(
      $.heading,
      'Requirements',
      $._newline,
      repeat($.requirement)
    ),

    requirement: $ => seq(
      $.subheading,
      field('id', $.requirement_id),
      ':',
      field('title', $.prose),
      $._newline,
      optional($.user_story),
      repeat($.ears_clause),
      optional($.acceptance_block)
    ),

    requirement_id: $ => /REQ-[A-Z]*-?\d+/,

    user_story: $ => seq(
      'As a',
      $.prose,
      ',',
      'I want',
      $.prose,
      optional(seq('so that', $.prose)),
      $._newline
    ),

    ears_clause: $ => seq(
      choice('when:', 'while:', 'if:', 'where:'),
      $.prose,
      $._newline,
      'the system shall:',
      $.prose_or_hole,
      $._newline
    ),

    acceptance_block: $ => seq(
      'acceptance:',
      $._newline,
      $._indent,
      repeat1($.acceptance_criterion),
      $._dedent
    ),

    acceptance_criterion: $ => seq(
      'given:',
      $.prose,
      $._newline,
      'when:',
      $.prose,
      $._newline,
      repeat1(seq('then:', $.prose, $._newline))
    ),

    // === CONCEPTS ===
    concepts_section: $ => seq(
      $.heading,
      'Concepts',
      $._newline,
      repeat($.concept)
    ),

    concept: $ => seq(
      'Concept',
      field('name', $.identifier),
      ':',
      $._newline,
      $._indent,
      optional($.prose),
      optional($._newline),
      repeat(choice($.field_def, $.one_of)),
      $._dedent
    ),

    field_def: $ => seq(
      'field',
      field('name', $.identifier),
      optional($.type_annotation),
      optional(seq(':', $.field_constraints)),
      $._newline
    ),

    one_of: $ => seq(
      'one of:',
      $.identifier_list,
      $._newline
    ),

    type_annotation: $ => seq(
      '(',
      $.type_reference,
      repeat(seq(' ', $.type_reference)),
      ')'
    ),

    // === BEHAVIORS ===
    behaviors_section: $ => seq(
      $.heading,
      'Behaviors',
      $._newline,
      repeat($.behavior)
    ),

    behavior: $ => seq(
      'Behavior',
      field('name', $.identifier),
      ':',
      $._newline,
      $._indent,
      optional($.implements_clause),
      optional(seq($.prose, $._newline)),
      optional($.given_block),
      optional($.returns_clause),
      optional($.requires_block),
      optional($.ensures_block),
      repeat($.example_block),
      $._dedent
    ),

    implements_clause: $ => seq(
      'Implements',
      $.requirement_id,
      repeat(seq(',', $.requirement_id)),
      '.',
      $._newline
    ),

    given_block: $ => seq(
      'given:',
      $._newline,
      $._indent,
      repeat1($.parameter),
      $._dedent
    ),

    parameter: $ => seq(
      field('name', $.identifier),
      $.type_annotation,
      $._newline
    ),

    returns_clause: $ => seq(
      'returns:',
      $.type_reference,
      optional(seq('or', $.type_reference)),
      $._newline
    ),

    requires_block: $ => seq(
      'requires:',
      $._newline,
      $._indent,
      repeat1(seq($.predicate, $._newline)),
      $._dedent
    ),

    ensures_block: $ => seq(
      'ensures:',
      $._newline,
      $._indent,
      repeat1(seq($.predicate_or_hole, $._newline)),
      $._dedent
    ),

    // === TYPED HOLES ===
    hole: $ => choice(
      $.untyped_hole,
      $.typed_hole,
      $.named_typed_hole
    ),

    untyped_hole: $ => seq(
      '[?',
      optional($.prose),
      ']'
    ),

    typed_hole: $ => seq(
      '[?',
      $.hole_signature,
      repeat($.hole_clause),
      ']'
    ),

    named_typed_hole: $ => seq(
      '[?',
      field('name', $.identifier),
      ':',
      $.hole_signature,
      repeat($.hole_clause),
      ']'
    ),

    hole_signature: $ => seq(
      $.type_reference,
      optional(seq('->', $.type_reference))
    ),

    hole_clause: $ => choice(
      seq('where:', $.predicate),
      seq('involving:', $.reference_list)
    ),

    // === TASKS ===
    tasks_section: $ => seq(
      $.heading,
      'Tasks',
      $._newline,
      repeat($.task)
    ),

    task: $ => seq(
      $.subheading,
      field('id', $.task_id),
      ':',
      field('title', $.prose),
      optional($.requirement_refs),
      $._newline,
      optional($.task_metadata)
    ),

    task_id: $ => /TASK-[A-Z]*-?\d+/,

    requirement_refs: $ => seq(
      '[',
      $.requirement_id,
      repeat(seq(',', $.requirement_id)),
      ']'
    ),

    task_metadata: $ => seq(
      $._indent,
      repeat1($.task_field),
      $._dedent
    ),

    task_field: $ => seq(
      choice('file:', 'tests:', 'depends:', 'status:'),
      $.prose,
      $._newline
    ),

    // === PRIMITIVES ===
    heading: $ => /#\s+/,
    subheading: $ => /##\s+/,
    
    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    identifier_list: $ => seq(
      $.identifier,
      repeat(seq(',', $.identifier))
    ),

    type_reference: $ => seq(
      '`',
      $.identifier,
      repeat(seq(' ', $.identifier)),
      '`'
    ),

    reference_list: $ => seq(
      $.type_reference,
      repeat(seq(',', $.type_reference))
    ),

    predicate: $ => /[^\n\[\]]+/,
    predicate_or_hole: $ => choice($.predicate, $.hole),
    prose: $ => /[^\n\[\]`]+/,
    prose_or_hole: $ => choice($.prose, $.hole),
    
    field_constraints: $ => repeat1(choice(
      'unique',
      'optional',
      seq('default:', $.type_reference),
      seq('at least', /\d+/, 'character'),
      $.prose
    )),
  }
});
```

**External Scanner**: `tree-sitter-topos/src/scanner.c`

```c
#include "tree_sitter/parser.h"

enum TokenType {
  INDENT,
  DEDENT,
  NEWLINE,
};

typedef struct {
  uint32_t indent_length;
  uint32_t indent_stack[32];
  uint8_t indent_stack_size;
} Scanner;

// Implementation follows tree-sitter-python pattern
// ...
```

**Deliverables**:
- [ ] Complete grammar.js with all Topos constructs
- [ ] External scanner for indentation (scanner.c)
- [ ] Syntax highlighting queries (highlights.scm)
- [ ] Test corpus with edge cases
- [ ] Rust bindings generated

### 1.2 AST Types with facet (Week 2)

**File**: `crates/topos-syntax/src/ast.rs`

```rust
//! AST types for Topos specifications.
//! 
//! All types derive `Facet` for unified reflection, serialization, and diffing.

use facet::Facet;
use std::sync::Arc;

/// Unique identifier for AST nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Facet)]
pub struct NodeId(pub u32);

/// Source location span
#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

/// Root of a Topos source file
#[derive(Debug, Clone, Facet)]
pub struct SourceFile {
    pub spec_declaration: Option<SpecDeclaration>,
    pub imports: Vec<Import>,
    pub sections: Vec<Section>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct SpecDeclaration {
    pub name: Identifier,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Import {
    pub path: String,
    pub items: ImportItems,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub enum ImportItems {
    Named(Vec<ImportItem>),
    Glob,
    Module { alias: Identifier },
}

#[derive(Debug, Clone, Facet)]
pub struct ImportItem {
    pub name: Identifier,
    #[facet(skip_if = "Option::is_none")]
    pub alias: Option<Identifier>,
}

// === SECTIONS ===

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
pub struct PrinciplesSection {
    pub principles: Vec<Principle>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Principle {
    pub name: Identifier,
    pub description: String,
    pub span: Span,
}

// === REQUIREMENTS ===

#[derive(Debug, Clone, Facet)]
pub struct RequirementsSection {
    pub requirements: Vec<Requirement>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Requirement {
    pub id: RequirementId,
    pub title: String,
    #[facet(skip_if = "Option::is_none")]
    pub user_story: Option<UserStory>,
    #[facet(skip_if = "Vec::is_empty")]
    pub ears_clauses: Vec<EarsClause>,
    #[facet(skip_if = "Option::is_none")]
    pub acceptance: Option<AcceptanceBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct RequirementId(pub String);

#[derive(Debug, Clone, Facet)]
pub struct UserStory {
    pub role: String,
    pub goal: String,
    #[facet(skip_if = "Option::is_none")]
    pub benefit: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct EarsClause {
    pub trigger: EarsTrigger,
    pub condition: String,
    pub behavior: ProseOrHole,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
pub enum EarsTrigger {
    When,
    While,
    If,
    Where,
}

#[derive(Debug, Clone, Facet)]
pub struct AcceptanceBlock {
    pub criteria: Vec<AcceptanceCriterion>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct AcceptanceCriterion {
    pub given: String,
    pub when: String,
    pub then: Vec<String>,
    pub span: Span,
}

// === CONCEPTS ===

#[derive(Debug, Clone, Facet)]
pub struct ConceptsSection {
    pub concepts: Vec<Concept>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Concept {
    pub name: Identifier,
    #[facet(skip_if = "Option::is_none")]
    pub doc: Option<String>,
    #[facet(skip_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    #[facet(skip_if = "Option::is_none")]
    pub one_of: Option<OneOf>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Field {
    pub name: Identifier,
    #[facet(skip_if = "Option::is_none")]
    pub type_expr: Option<TypeExpr>,
    #[facet(skip_if = "Vec::is_empty")]
    pub constraints: Vec<FieldConstraint>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub enum FieldConstraint {
    Unique,
    Optional,
    Default(TypeReference),
    MinLength(u32),
    Custom(String),
}

#[derive(Debug, Clone, Facet)]
pub struct OneOf {
    pub variants: Vec<Identifier>,
    pub span: Span,
}

// === BEHAVIORS ===

#[derive(Debug, Clone, Facet)]
pub struct BehaviorsSection {
    pub behaviors: Vec<Behavior>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Behavior {
    pub name: Identifier,
    #[facet(skip_if = "Vec::is_empty")]
    pub implements: Vec<RequirementId>,
    #[facet(skip_if = "Option::is_none")]
    pub doc: Option<String>,
    #[facet(skip_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    #[facet(skip_if = "Option::is_none")]
    pub returns: Option<ReturnsClause>,
    #[facet(skip_if = "Vec::is_empty")]
    pub requires: Vec<Predicate>,
    #[facet(skip_if = "Vec::is_empty")]
    pub ensures: Vec<PredicateOrHole>,
    #[facet(skip_if = "Vec::is_empty")]
    pub ears_clauses: Vec<EarsClause>,
    #[facet(skip_if = "Vec::is_empty")]
    pub examples: Vec<ExampleBlock>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Parameter {
    pub name: Identifier,
    pub type_expr: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct ReturnsClause {
    pub success_type: TypeExpr,
    #[facet(skip_if = "Option::is_none")]
    pub error_type: Option<TypeExpr>,
    pub span: Span,
}

// === TYPED HOLES ===

/// Unique identifier for holes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Facet)]
pub struct HoleId(pub u32);

#[derive(Debug, Clone, Facet)]
pub struct TypedHole {
    pub id: HoleId,
    #[facet(skip_if = "Option::is_none")]
    pub name: Option<Identifier>,
    #[facet(skip_if = "Option::is_none")]
    pub signature: Option<HoleSignature>,
    #[facet(skip_if = "Vec::is_empty")]
    pub constraints: Vec<HoleConstraint>,
    #[facet(skip_if = "Vec::is_empty")]
    pub involving: Vec<TypeReference>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct HoleSignature {
    #[facet(skip_if = "Option::is_none")]
    pub input: Option<TypeExpr>,
    #[facet(skip_if = "Option::is_none")]
    pub output: Option<TypeExpr>,
}

#[derive(Debug, Clone, Facet)]
pub enum HoleConstraint {
    Where(Predicate),
    Involving(Vec<TypeReference>),
}

// === TASKS ===

#[derive(Debug, Clone, Facet)]
pub struct TasksSection {
    pub tasks: Vec<Task>,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    #[facet(skip_if = "Vec::is_empty")]
    pub requirement_refs: Vec<RequirementId>,
    #[facet(skip_if = "Option::is_none")]
    pub file: Option<String>,
    #[facet(skip_if = "Option::is_none")]
    pub tests: Option<String>,
    #[facet(skip_if = "Vec::is_empty")]
    pub depends: Vec<TaskId>,
    pub status: TaskStatus,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct TaskId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Facet)]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Blocked,
}

// === TYPE EXPRESSIONS ===

#[derive(Debug, Clone, Facet)]
pub struct TypeExpr {
    pub parts: Vec<TypeReference>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct TypeReference {
    pub name: String,
    pub span: Span,
}

// === UTILITY TYPES ===

#[derive(Debug, Clone, PartialEq, Eq, Hash, Facet)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct Predicate {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub enum ProseOrHole {
    Prose(String),
    Hole(TypedHole),
}

#[derive(Debug, Clone, Facet)]
pub enum PredicateOrHole {
    Predicate(Predicate),
    Hole(TypedHole),
}

#[derive(Debug, Clone, Facet)]
pub struct ExampleBlock {
    pub title: Option<String>,
    pub given: Vec<String>,
    pub when: String,
    pub then: Vec<String>,
    pub span: Span,
}
```

### 1.3 Parser Implementation (Weeks 3-4)

**File**: `crates/topos-syntax/src/parser.rs`

```rust
//! Converts tree-sitter CST to Topos AST.

use crate::ast::*;
use facet_reflect::Partial;
use tree_sitter::{Node, Parser, Tree};

pub struct ToposParser {
    parser: Parser,
    next_hole_id: u32,
}

impl ToposParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_topos::LANGUAGE.into())
            .expect("Failed to load Topos grammar");
        
        Self {
            parser,
            next_hole_id: 0,
        }
    }
    
    pub fn parse(&mut self, source: &str) -> ParseResult {
        let tree = self.parser.parse(source, None)
            .expect("Parser returned None");
        
        let mut errors = Vec::new();
        let ast = self.build_ast(&tree, source, &mut errors);
        
        ParseResult { ast, errors }
    }
    
    pub fn parse_incremental(&mut self, source: &str, old_tree: &Tree) -> ParseResult {
        let tree = self.parser.parse(source, Some(old_tree))
            .expect("Parser returned None");
        
        let mut errors = Vec::new();
        let ast = self.build_ast(&tree, source, &mut errors);
        
        ParseResult { ast, errors }
    }
    
    fn build_ast(&mut self, tree: &Tree, source: &str, errors: &mut Vec<ParseError>) -> SourceFile {
        let root = tree.root_node();
        
        // Collect any syntax errors
        self.collect_errors(root, source, errors);
        
        // Build AST from CST
        let mut spec_declaration = None;
        let mut imports = Vec::new();
        let mut sections = Vec::new();
        
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "spec_declaration" => {
                    spec_declaration = Some(self.build_spec_declaration(child, source));
                }
                "import" => {
                    imports.push(self.build_import(child, source));
                }
                "principles_section" | "requirements_section" | 
                "design_section" | "concepts_section" | 
                "behaviors_section" | "tasks_section" => {
                    if let Some(section) = self.build_section(child, source, errors) {
                        sections.push(section);
                    }
                }
                _ => {}
            }
        }
        
        SourceFile {
            spec_declaration,
            imports,
            sections,
            span: self.span(root),
        }
    }
    
    fn build_section(
        &mut self, 
        node: Node, 
        source: &str, 
        errors: &mut Vec<ParseError>
    ) -> Option<Section> {
        match node.kind() {
            "principles_section" => {
                Some(Section::Principles(self.build_principles_section(node, source)))
            }
            "requirements_section" => {
                Some(Section::Requirements(self.build_requirements_section(node, source, errors)))
            }
            "concepts_section" => {
                Some(Section::Concepts(self.build_concepts_section(node, source)))
            }
            "behaviors_section" => {
                Some(Section::Behaviors(self.build_behaviors_section(node, source, errors)))
            }
            "tasks_section" => {
                Some(Section::Tasks(self.build_tasks_section(node, source)))
            }
            _ => None,
        }
    }
    
    fn build_typed_hole(&mut self, node: Node, source: &str) -> TypedHole {
        let id = HoleId(self.next_hole_id);
        self.next_hole_id += 1;
        
        let name = node.child_by_field_name("name")
            .map(|n| self.build_identifier(n, source));
        
        let signature = self.find_child(node, "hole_signature")
            .map(|n| self.build_hole_signature(n, source));
        
        let mut constraints = Vec::new();
        let mut involving = Vec::new();
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "hole_clause" => {
                    if let Some(constraint) = self.build_hole_clause(child, source) {
                        match constraint {
                            HoleConstraint::Where(_) => constraints.push(constraint),
                            HoleConstraint::Involving(refs) => involving.extend(refs),
                        }
                    }
                }
                _ => {}
            }
        }
        
        TypedHole {
            id,
            name,
            signature,
            constraints,
            involving,
            span: self.span(node),
        }
    }
    
    // Helper methods
    
    fn span(&self, node: Node) -> Span {
        Span {
            start: Position {
                line: node.start_position().row as u32,
                column: node.start_position().column as u32,
                offset: node.start_byte() as u32,
            },
            end: Position {
                line: node.end_position().row as u32,
                column: node.end_position().column as u32,
                offset: node.end_byte() as u32,
            },
        }
    }
    
    fn text<'a>(&self, node: Node, source: &'a str) -> &'a str {
        &source[node.start_byte()..node.end_byte()]
    }
    
    fn find_child<'a>(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        let mut cursor = node.walk();
        node.children(&mut cursor).find(|c| c.kind() == kind)
    }
    
    fn collect_errors(&self, node: Node, source: &str, errors: &mut Vec<ParseError>) {
        if node.is_error() || node.is_missing() {
            errors.push(ParseError {
                message: if node.is_missing() {
                    format!("Missing {}", node.kind())
                } else {
                    format!("Syntax error: unexpected '{}'", self.text(node, source))
                },
                span: self.span(node),
            });
        }
        
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_errors(child, source, errors);
        }
    }
    
    // ... additional build methods for each AST node type
}

#[derive(Debug)]
pub struct ParseResult {
    pub ast: SourceFile,
    pub errors: Vec<ParseError>,
}

#[derive(Debug, Clone, Facet)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}
```

**Deliverables**:
- [ ] Complete parser with all AST nodes
- [ ] Error recovery (always produces AST)
- [ ] Incremental parsing support
- [ ] Unit tests for all constructs
- [ ] Snapshot tests for error messages

---

## Phase 2: Semantic Analysis (Weeks 5-8)

### 2.1 Salsa Database Setup (Week 5)

**File**: `crates/topos-analysis/src/db.rs`

```rust
use salsa::Durability;
use std::sync::Arc;
use crate::*;

#[salsa::db]
pub trait ToposDatabase: salsa::Database {
    // === FILE INPUTS ===
    
    #[salsa::input]
    fn file_text(&self, file: FileId) -> Arc<str>;
    
    #[salsa::input]  
    fn file_path(&self, file: FileId) -> Arc<std::path::Path>;
    
    // === WORKSPACE ===
    
    #[salsa::input]
    fn workspace_root(&self) -> Arc<std::path::Path>;
    
    #[salsa::tracked]
    fn workspace_files(&self) -> Arc<Vec<FileId>>;
    
    #[salsa::tracked]
    fn project_config(&self) -> Arc<ProjectConfig>;
    
    // === PARSING ===
    
    #[salsa::tracked]
    fn parse(&self, file: FileId) -> Arc<ParseResult>;
    
    #[salsa::tracked]
    fn ast(&self, file: FileId) -> Arc<SourceFile>;
    
    // === NAME RESOLUTION ===
    
    #[salsa::tracked]
    fn file_scope(&self, file: FileId) -> Arc<Scope>;
    
    #[salsa::tracked]
    fn imports(&self, file: FileId) -> Arc<ImportMap>;
    
    #[salsa::tracked]
    fn exports(&self, file: FileId) -> Arc<ExportMap>;
    
    #[salsa::tracked]
    fn resolve_reference(&self, file: FileId, reference: Reference) -> Option<Definition>;
    
    // === TYPED HOLES ===
    
    #[salsa::tracked]
    fn file_holes(&self, file: FileId) -> Arc<Vec<TypedHole>>;
    
    #[salsa::tracked]
    fn hole_context(&self, file: FileId, hole: HoleId) -> Arc<HoleContext>;
    
    // === TRACEABILITY ===
    
    #[salsa::tracked]
    fn file_requirements(&self, file: FileId) -> Arc<Vec<RequirementId>>;
    
    #[salsa::tracked]
    fn requirement_implementations(&self, req: RequirementId) -> Arc<Vec<BehaviorId>>;
    
    #[salsa::tracked]
    fn requirement_tasks(&self, req: RequirementId) -> Arc<Vec<TaskId>>;
    
    #[salsa::tracked]
    fn traceability_report(&self) -> Arc<TraceabilityReport>;
    
    // === DIAGNOSTICS ===
    
    #[salsa::tracked]
    fn file_diagnostics(&self, file: FileId) -> Arc<Vec<Diagnostic>>;
    
    #[salsa::tracked]
    fn workspace_diagnostics(&self) -> Arc<Vec<Diagnostic>>;
}

#[salsa::db]
#[derive(Default)]
pub struct RootDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for RootDatabase {
    fn salsa_event(&self, event: &dyn Fn() -> salsa::Event) {
        let event = event();
        tracing::trace!("salsa event: {:?}", event);
    }
}

impl ToposDatabase for RootDatabase {}

impl RootDatabase {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Load standard library with HIGH durability
    pub fn load_stdlib(&mut self, files: impl IntoIterator<Item = (FileId, String)>) {
        for (file, content) in files {
            self.set_file_text_with_durability(
                file,
                Arc::from(content),
                Durability::HIGH,
            );
        }
    }
    
    /// Update a user file (LOW durability)
    pub fn update_file(&mut self, file: FileId, content: String) {
        self.set_file_text_with_durability(
            file,
            Arc::from(content),
            Durability::LOW,
        );
    }
}
```

### 2.2 Name Resolution (Week 6)

**File**: `crates/topos-analysis/src/resolve.rs`

```rust
use crate::db::ToposDatabase;
use topos_syntax::ast::*;
use facet::Facet;

#[derive(Debug, Clone, Facet)]
pub struct Scope {
    pub definitions: Vec<Definition>,
    pub parent: Option<ScopeId>,
}

#[derive(Debug, Clone, Facet)]
pub struct Definition {
    pub name: String,
    pub kind: DefinitionKind,
    pub file: FileId,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub enum DefinitionKind {
    Concept,
    Behavior,
    Requirement,
    Task,
    Field { parent: Identifier },
    Parameter { parent: Identifier },
    TypeAlias,
    Import { original: String },
}

#[derive(Debug, Clone, Facet)]
pub struct Reference {
    pub name: String,
    pub span: Span,
}

pub fn resolve_references(
    db: &dyn ToposDatabase,
    file: FileId,
) -> Vec<(Reference, Option<Definition>)> {
    let ast = db.ast(file);
    let scope = db.file_scope(file);
    let imports = db.imports(file);
    
    let mut results = Vec::new();
    
    // Walk AST and collect all references
    for reference in collect_references(&ast) {
        let definition = resolve_single(db, &reference, &scope, &imports);
        results.push((reference, definition));
    }
    
    results
}

fn resolve_single(
    db: &dyn ToposDatabase,
    reference: &Reference,
    scope: &Scope,
    imports: &ImportMap,
) -> Option<Definition> {
    // 1. Check local scope
    if let Some(def) = scope.definitions.iter()
        .find(|d| d.name == reference.name) 
    {
        return Some(def.clone());
    }
    
    // 2. Check imports
    if let Some(import) = imports.get(&reference.name) {
        let target_file = db.file_from_path(&import.source)?;
        let target_exports = db.exports(target_file);
        return target_exports.get(&import.original_name).cloned();
    }
    
    // 3. Check parent scope
    if let Some(parent_id) = scope.parent {
        let parent_scope = db.scope(parent_id);
        return resolve_single(db, reference, &parent_scope, imports);
    }
    
    None
}
```

### 2.3 Hole Analysis (Week 7)

**File**: `crates/topos-analysis/src/holes.rs`

```rust
use crate::db::ToposDatabase;
use topos_syntax::ast::*;
use facet::Facet;

/// Context information for a typed hole
#[derive(Debug, Clone, Facet)]
pub struct HoleContext {
    pub hole: TypedHole,
    pub enclosing_behavior: Option<BehaviorId>,
    pub available_symbols: Vec<Symbol>,
    pub type_constraints: Vec<TypeConstraint>,
    pub semantic_constraints: Vec<SemanticConstraint>,
}

#[derive(Debug, Clone, Facet)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_expr: Option<TypeExpr>,
}

#[derive(Debug, Clone, Facet)]
pub enum SymbolKind {
    Parameter,
    LocalBinding,
    Concept,
    Field { concept: String },
}

#[derive(Debug, Clone, Facet)]
pub struct TypeConstraint {
    pub kind: TypeConstraintKind,
    pub source: ConstraintSource,
}

#[derive(Debug, Clone, Facet)]
pub enum TypeConstraintKind {
    InputType(TypeExpr),
    OutputType(TypeExpr),
    MustImplement(String),
}

#[derive(Debug, Clone, Facet)]
pub struct SemanticConstraint {
    pub predicate: String,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, Copy, Facet)]
pub enum ConstraintSource {
    HoleSignature,
    WhereClause,
    EnclosingBehavior,
    InferredFromUsage,
}

pub fn analyze_hole(
    db: &dyn ToposDatabase,
    file: FileId,
    hole_id: HoleId,
) -> HoleContext {
    let ast = db.ast(file);
    let hole = find_hole(&ast, hole_id).expect("Hole not found");
    
    // Find enclosing behavior
    let enclosing_behavior = find_enclosing_behavior(&ast, hole.span);
    
    // Collect available symbols in scope
    let available_symbols = collect_symbols_in_scope(db, file, hole.span);
    
    // Extract type constraints
    let mut type_constraints = Vec::new();
    
    if let Some(sig) = &hole.signature {
        if let Some(input) = &sig.input {
            type_constraints.push(TypeConstraint {
                kind: TypeConstraintKind::InputType(input.clone()),
                source: ConstraintSource::HoleSignature,
            });
        }
        if let Some(output) = &sig.output {
            type_constraints.push(TypeConstraint {
                kind: TypeConstraintKind::OutputType(output.clone()),
                source: ConstraintSource::HoleSignature,
            });
        }
    }
    
    // Extract semantic constraints from where clauses
    let semantic_constraints: Vec<_> = hole.constraints.iter()
        .filter_map(|c| match c {
            HoleConstraint::Where(pred) => Some(SemanticConstraint {
                predicate: pred.text.clone(),
                references: extract_references(&pred.text),
            }),
            _ => None,
        })
        .collect();
    
    HoleContext {
        hole: hole.clone(),
        enclosing_behavior,
        available_symbols,
        type_constraints,
        semantic_constraints,
    }
}

/// Check if a proposed fill is compatible with the hole
pub fn check_hole_compatibility(
    context: &HoleContext,
    proposed_type: &TypeExpr,
) -> CompatibilityResult {
    let mut errors = Vec::new();
    
    for constraint in &context.type_constraints {
        match &constraint.kind {
            TypeConstraintKind::InputType(expected) => {
                if !is_subtype(proposed_type, expected) {
                    errors.push(CompatibilityError {
                        message: format!(
                            "Input type {} is not compatible with expected {}",
                            proposed_type, expected
                        ),
                        source: constraint.source,
                    });
                }
            }
            TypeConstraintKind::OutputType(expected) => {
                if !is_subtype(expected, proposed_type) {
                    errors.push(CompatibilityError {
                        message: format!(
                            "Output type {} is not compatible with expected {}",
                            proposed_type, expected
                        ),
                        source: constraint.source,
                    });
                }
            }
            _ => {}
        }
    }
    
    if errors.is_empty() {
        CompatibilityResult::Compatible
    } else {
        CompatibilityResult::Incompatible(errors)
    }
}

#[derive(Debug)]
pub enum CompatibilityResult {
    Compatible,
    Incompatible(Vec<CompatibilityError>),
}

#[derive(Debug, Facet)]
pub struct CompatibilityError {
    pub message: String,
    pub source: ConstraintSource,
}
```

### 2.4 Traceability Analysis (Week 8)

**File**: `crates/topos-analysis/src/traceability.rs`

```rust
use crate::db::ToposDatabase;
use topos_syntax::ast::*;
use facet::Facet;
use std::collections::HashMap;

#[derive(Debug, Clone, Facet)]
pub struct TraceabilityReport {
    pub requirements: Vec<RequirementTrace>,
    pub orphan_behaviors: Vec<BehaviorId>,
    pub orphan_tasks: Vec<TaskId>,
    pub coverage: CoverageStats,
}

#[derive(Debug, Clone, Facet)]
pub struct RequirementTrace {
    pub requirement: RequirementId,
    pub title: String,
    pub behaviors: Vec<BehaviorTrace>,
    pub tasks: Vec<TaskTrace>,
    pub coverage: RequirementCoverage,
}

#[derive(Debug, Clone, Facet)]
pub struct BehaviorTrace {
    pub id: BehaviorId,
    pub name: String,
    pub file: FileId,
    pub span: Span,
}

#[derive(Debug, Clone, Facet)]
pub struct TaskTrace {
    pub id: TaskId,
    pub title: String,
    pub file: Option<String>,
    pub tests: Option<String>,
    pub status: TaskStatus,
    pub file_id: FileId,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Facet)]
pub struct RequirementCoverage {
    pub has_behavior: bool,
    pub has_task: bool,
    pub has_implementation: bool,
    pub has_tests: bool,
}

#[derive(Debug, Clone, Facet)]
pub struct CoverageStats {
    pub total_requirements: u32,
    pub with_behaviors: u32,
    pub with_tasks: u32,
    pub with_implementation: u32,
    pub with_tests: u32,
}

pub fn compute_traceability(db: &dyn ToposDatabase) -> TraceabilityReport {
    let files = db.workspace_files();
    
    // Collect all requirements, behaviors, and tasks
    let mut requirements: HashMap<RequirementId, RequirementTrace> = HashMap::new();
    let mut behavior_to_reqs: HashMap<BehaviorId, Vec<RequirementId>> = HashMap::new();
    let mut task_to_reqs: HashMap<TaskId, Vec<RequirementId>> = HashMap::new();
    
    for file in files.iter() {
        let ast = db.ast(*file);
        
        // Collect requirements
        for section in &ast.sections {
            if let Section::Requirements(req_section) = section {
                for req in &req_section.requirements {
                    requirements.insert(req.id.clone(), RequirementTrace {
                        requirement: req.id.clone(),
                        title: req.title.clone(),
                        behaviors: Vec::new(),
                        tasks: Vec::new(),
                        coverage: RequirementCoverage {
                            has_behavior: false,
                            has_task: false,
                            has_implementation: false,
                            has_tests: false,
                        },
                    });
                }
            }
        }
        
        // Collect behaviors and their implements clauses
        for section in &ast.sections {
            if let Section::Behaviors(beh_section) = section {
                for behavior in &beh_section.behaviors {
                    let beh_id = BehaviorId::from_name(&behavior.name.name);
                    for req_id in &behavior.implements {
                        behavior_to_reqs
                            .entry(beh_id.clone())
                            .or_default()
                            .push(req_id.clone());
                        
                        if let Some(trace) = requirements.get_mut(req_id) {
                            trace.behaviors.push(BehaviorTrace {
                                id: beh_id.clone(),
                                name: behavior.name.name.clone(),
                                file: *file,
                                span: behavior.span,
                            });
                            trace.coverage.has_behavior = true;
                        }
                    }
                }
            }
        }
        
        // Collect tasks and their requirement refs
        for section in &ast.sections {
            if let Section::Tasks(task_section) = section {
                for task in &task_section.tasks {
                    for req_id in &task.requirement_refs {
                        task_to_reqs
                            .entry(task.id.clone())
                            .or_default()
                            .push(req_id.clone());
                        
                        if let Some(trace) = requirements.get_mut(req_id) {
                            trace.tasks.push(TaskTrace {
                                id: task.id.clone(),
                                title: task.title.clone(),
                                file: task.file.clone(),
                                tests: task.tests.clone(),
                                status: task.status,
                                file_id: *file,
                                span: task.span,
                            });
                            trace.coverage.has_task = true;
                            trace.coverage.has_implementation = task.file.is_some();
                            trace.coverage.has_tests = task.tests.is_some();
                        }
                    }
                }
            }
        }
    }
    
    // Find orphans
    let orphan_behaviors: Vec<_> = behavior_to_reqs.iter()
        .filter(|(_, reqs)| reqs.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    
    let orphan_tasks: Vec<_> = task_to_reqs.iter()
        .filter(|(_, reqs)| reqs.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    
    // Compute coverage stats
    let total = requirements.len() as u32;
    let with_behaviors = requirements.values()
        .filter(|r| r.coverage.has_behavior)
        .count() as u32;
    let with_tasks = requirements.values()
        .filter(|r| r.coverage.has_task)
        .count() as u32;
    let with_impl = requirements.values()
        .filter(|r| r.coverage.has_implementation)
        .count() as u32;
    let with_tests = requirements.values()
        .filter(|r| r.coverage.has_tests)
        .count() as u32;
    
    TraceabilityReport {
        requirements: requirements.into_values().collect(),
        orphan_behaviors,
        orphan_tasks,
        coverage: CoverageStats {
            total_requirements: total,
            with_behaviors,
            with_tasks,
            with_implementation: with_impl,
            with_tests,
        },
    }
}
```

---

## Phase 3: LSP Server (Weeks 9-12)

### 3.1 Server Setup (Week 9)

**File**: `crates/topos-lsp/src/server.rs`

```rust
use tower_lsp_server::{
    jsonrpc::Result,
    lsp_types::*,
    Client, LanguageServer, LspService, Server,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use topos_analysis::{RootDatabase, ToposDatabase};

pub struct ToposLanguageServer {
    client: Client,
    db: Arc<RwLock<RootDatabase>>,
}

impl ToposLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            db: Arc::new(RwLock::new(RootDatabase::new())),
        }
    }
}

impl LanguageServer for ToposLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Initialize workspace
        if let Some(folders) = params.workspace_folders {
            let mut db = self.db.write().await;
            for folder in folders {
                db.set_workspace_root(Arc::from(folder.uri.to_file_path().unwrap()));
            }
        }
        
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["`".to_string(), "[".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        inter_file_dependencies: true,
                        workspace_diagnostics: true,
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "topos-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Topos LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        
        let mut db = self.db.write().await;
        let file = db.file_from_uri(&uri);
        db.update_file(file, text);
        drop(db);
        
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        
        let mut db = self.db.write().await;
        let file = db.file_from_uri(&uri);
        
        // Apply incremental changes
        let mut text = db.file_text(file).to_string();
        for change in params.content_changes {
            if let Some(range) = change.range {
                // Convert LSP range to byte offsets and apply
                let start = position_to_offset(&text, range.start);
                let end = position_to_offset(&text, range.end);
                text.replace_range(start..end, &change.text);
            } else {
                text = change.text;
            }
        }
        
        db.update_file(file, text);
        drop(db);
        
        self.publish_diagnostics(uri).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let db = self.db.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        
        let file = db.file_from_uri(uri);
        Ok(handlers::hover::handle_hover(&*db, file, pos))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let db = self.db.read().await;
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        
        let file = db.file_from_uri(uri);
        Ok(handlers::goto::handle_goto_definition(&*db, file, pos))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let db = self.db.read().await;
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        
        let file = db.file_from_uri(uri);
        Ok(handlers::references::handle_references(&*db, file, pos))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let db = self.db.read().await;
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        
        let file = db.file_from_uri(uri);
        Ok(handlers::completion::handle_completion(&*db, file, pos))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let db = self.db.read().await;
        let uri = &params.text_document.uri;
        let range = params.range;
        
        let file = db.file_from_uri(uri);
        Ok(handlers::actions::handle_code_actions(&*db, file, range))
    }
}

impl ToposLanguageServer {
    async fn publish_diagnostics(&self, uri: Url) {
        let db = self.db.read().await;
        let file = db.file_from_uri(&uri);
        let diagnostics = db.file_diagnostics(file);
        
        let lsp_diagnostics: Vec<Diagnostic> = diagnostics.iter()
            .map(|d| d.to_lsp_diagnostic())
            .collect();
        
        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ToposLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

### 3.2-3.4 LSP Handlers (Weeks 10-12)

See ARCHITECTURE.md for detailed handler implementations using `Peek` for hover and `facet-diff` for code actions.

**Deliverables**:
- [ ] Hover with typed hole expansion
- [ ] Go-to-definition across files
- [ ] Find all references
- [ ] Completion for types and references
- [ ] Code actions for hole filling
- [ ] Diagnostics with quick fixes
- [ ] Document symbols
- [ ] Workspace symbols

---

## Phase 4: AI Integration (Weeks 13-16)

### 4.1 MCP Server (Weeks 13-14)

**File**: `crates/topos-mcp/src/server.rs`

```rust
use rmcp::{ServerBuilder, tool, ToolHandler, ToolResult};
use topos_analysis::{RootDatabase, ToposDatabase};
use topos_diff::{compare_models, extract_model_from_code, extract_model_from_spec};
use facet_json::ToJson;
use std::sync::Arc;

pub struct ToposMcpServer {
    db: Arc<RootDatabase>,
}

impl ToposMcpServer {
    pub fn new(db: Arc<RootDatabase>) -> Self {
        Self { db }
    }
    
    pub async fn run(self) -> Result<(), rmcp::Error> {
        ServerBuilder::new("topos-mcp", env!("CARGO_PKG_VERSION"))
            .with_capability(rmcp::Capability::Tools)
            .with_tool(CreateSpecTool { db: self.db.clone() })
            .with_tool(GenerateCodeTool { db: self.db.clone() })
            .with_tool(ExtractSpecTool { db: self.db.clone() })
            .with_tool(CompleteHoleTool { db: self.db.clone() })
            .with_tool(AnalyzeSpecTool { db: self.db.clone() })
            .with_tool(TraceRequirementTool { db: self.db.clone() })
            .build()
            .run_stdio()
            .await
    }
}

struct CreateSpecTool {
    db: Arc<RootDatabase>,
}

#[tool(
    name = "create_spec",
    description = "Generate a Topos specification from natural language intent. Creates requirements, concepts, behaviors, and tasks."
)]
impl ToolHandler for CreateSpecTool {
    async fn call(
        &self,
        #[arg(description = "Natural language description of what you want to build")]
        intent: String,
        #[arg(description = "Target domain (e.g., 'users', 'orders', 'payments')")]
        domain: Option<String>,
        #[arg(description = "Include example acceptance criteria")]
        with_examples: Option<bool>,
        #[arg(description = "Existing spec file to extend")]
        extend_file: Option<String>,
    ) -> ToolResult {
        // Generate spec from intent
        let spec = generate_spec_from_intent(
            &intent,
            domain.as_deref(),
            with_examples.unwrap_or(true),
            extend_file.as_deref(),
        )?;
        
        Ok(ToolResult::success(serde_json::json!({
            "spec": spec,
            "file_suggestion": format!("specs/{}.tps", domain.unwrap_or("main".to_string())),
        })))
    }
}

struct ExtractSpecTool {
    db: Arc<RootDatabase>,
}

#[tool(
    name = "extract_spec",
    description = "Extract a Topos specification from existing code. Analyzes types, functions, and tests to generate concepts and behaviors."
)]
impl ToolHandler for ExtractSpecTool {
    async fn call(
        &self,
        #[arg(description = "Path to source file or directory")]
        source_path: String,
        #[arg(description = "Programming language (typescript, python, rust, go)")]
        language: String,
        #[arg(description = "Specific types/functions to extract (comma-separated)")]
        focus: Option<String>,
    ) -> ToolResult {
        let model = extract_model_from_code(&source_path, &language, focus.as_deref())?;
        
        let spec = generate_spec_from_model(&model)?;
        
        Ok(ToolResult::success(serde_json::json!({
            "spec": spec,
            "extracted_concepts": model.concepts.len(),
            "extracted_behaviors": model.behaviors.len(),
        })))
    }
}

struct CompleteHoleTool {
    db: Arc<RootDatabase>,
}

#[tool(
    name = "complete_hole",
    description = "Generate type-compatible completions for a typed hole based on its constraints and context."
)]
impl ToolHandler for CompleteHoleTool {
    async fn call(
        &self,
        #[arg(description = "Path to spec file containing the hole")]
        file: String,
        #[arg(description = "Hole identifier (e.g., 'payment_flow' or numeric ID)")]
        hole_id: String,
        #[arg(description = "Maximum number of suggestions (default: 5)")]
        max_suggestions: Option<u32>,
        #[arg(description = "Prefer simpler solutions")]
        prefer_simple: Option<bool>,
    ) -> ToolResult {
        let db = &*self.db;
        let file_id = db.file_from_path(&file).ok_or("File not found")?;
        
        let holes = db.file_holes(file_id);
        let hole = holes.iter()
            .find(|h| h.name.as_ref().map(|n| n.name.as_str()) == Some(&hole_id)
                || h.id.0.to_string() == hole_id)
            .ok_or("Hole not found")?;
        
        let context = db.hole_context(file_id, hole.id);
        let completions = generate_hole_completions(
            &context,
            max_suggestions.unwrap_or(5),
            prefer_simple.unwrap_or(false),
        )?;
        
        Ok(ToolResult::success(serde_json::json!({
            "hole": {
                "id": hole.id.0,
                "name": hole.name.as_ref().map(|n| &n.name),
                "signature": hole.signature,
            },
            "context": {
                "enclosing_behavior": context.enclosing_behavior,
                "available_symbols": context.available_symbols.len(),
                "constraints": context.type_constraints.len(),
            },
            "completions": completions,
        })))
    }
}

struct AnalyzeSpecTool {
    db: Arc<RootDatabase>,
}

#[tool(
    name = "analyze_spec",
    description = "Analyze a Topos specification for coverage, consistency, and potential issues."
)]
impl ToolHandler for AnalyzeSpecTool {
    async fn call(
        &self,
        #[arg(description = "Path to spec file or directory")]
        path: String,
        #[arg(description = "Include traceability analysis")]
        include_traceability: Option<bool>,
        #[arg(description = "Check for spec↔code drift")]
        check_drift: Option<bool>,
        #[arg(description = "Source code path for drift checking")]
        code_path: Option<String>,
    ) -> ToolResult {
        let db = &*self.db;
        
        let mut result = serde_json::json!({
            "diagnostics": [],
            "holes": [],
            "coverage": {},
        });
        
        // Collect diagnostics
        let diagnostics = db.workspace_diagnostics();
        result["diagnostics"] = serde_json::to_value(&*diagnostics)?;
        
        // Collect holes
        let files = db.workspace_files();
        let mut all_holes = Vec::new();
        for file in files.iter() {
            let holes = db.file_holes(*file);
            all_holes.extend(holes.iter().cloned());
        }
        result["holes"] = serde_json::to_value(&all_holes)?;
        
        // Traceability
        if include_traceability.unwrap_or(true) {
            let report = db.traceability_report();
            result["traceability"] = serde_json::to_value(&*report)?;
        }
        
        // Drift detection
        if check_drift.unwrap_or(false) {
            if let Some(code_path) = code_path {
                let spec_model = extract_model_from_spec(db)?;
                let code_model = extract_model_from_code(&code_path, "auto", None)?;
                let drift = compare_models(&spec_model, &code_model);
                result["drift"] = serde_json::to_value(&drift)?;
            }
        }
        
        Ok(ToolResult::success(result))
    }
}

struct TraceRequirementTool {
    db: Arc<RootDatabase>,
}

#[tool(
    name = "trace_requirement",
    description = "Trace a requirement through behaviors, tasks, and implementation files."
)]
impl ToolHandler for TraceRequirementTool {
    async fn call(
        &self,
        #[arg(description = "Requirement ID (e.g., 'REQ-USR-001')")]
        requirement_id: String,
    ) -> ToolResult {
        let db = &*self.db;
        let req_id = RequirementId(requirement_id);
        
        let behaviors = db.requirement_implementations(req_id.clone());
        let tasks = db.requirement_tasks(req_id.clone());
        
        Ok(ToolResult::success(serde_json::json!({
            "requirement": req_id.0,
            "behaviors": behaviors.iter().map(|b| b.0.clone()).collect::<Vec<_>>(),
            "tasks": tasks.iter().map(|t| {
                serde_json::json!({
                    "id": t.id.0,
                    "file": t.file,
                    "tests": t.tests,
                    "status": format!("{:?}", t.status),
                })
            }).collect::<Vec<_>>(),
        })))
    }
}
```

### 4.2 Bidirectional Sync Engine (Weeks 15-16)

**File**: `crates/topos-diff/src/sync.rs`

```rust
use crate::{compare_models, ComparisonResult, DriftChange, DomainModel};
use facet::Facet;
use facet_reflect::{Peek, check_same_report};

#[derive(Debug, Clone, Facet)]
pub struct SyncResult {
    pub status: SyncStatus,
    pub spec_changes: Vec<SpecChange>,
    pub code_changes: Vec<CodeChange>,
    pub conflicts: Vec<Conflict>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Facet)]
pub enum SyncStatus {
    InSync,
    SpecAhead,
    CodeAhead,
    Diverged,
}

#[derive(Debug, Clone, Facet)]
pub enum SpecChange {
    AddConcept { name: String, from_code: String },
    AddField { concept: String, field: String, type_expr: String },
    AddBehavior { name: String, signature: String },
    UpdateField { concept: String, field: String, new_type: String },
}

#[derive(Debug, Clone, Facet)]
pub enum CodeChange {
    AddType { name: String, from_spec: String },
    AddField { type_name: String, field: String, type_expr: String },
    AddFunction { name: String, signature: String },
    UpdateField { type_name: String, field: String, new_type: String },
}

#[derive(Debug, Clone, Facet)]
pub struct Conflict {
    pub location: String,
    pub spec_value: String,
    pub code_value: String,
    pub suggestion: ConflictResolution,
}

#[derive(Debug, Clone, Copy, Facet)]
pub enum ConflictResolution {
    KeepSpec,
    KeepCode,
    Manual,
}

pub fn compute_sync(
    spec_model: &DomainModel,
    code_model: &DomainModel,
) -> SyncResult {
    let comparison = compare_models(spec_model, code_model);
    
    match comparison {
        ComparisonResult::InSync => SyncResult {
            status: SyncStatus::InSync,
            spec_changes: Vec::new(),
            code_changes: Vec::new(),
            conflicts: Vec::new(),
        },
        ComparisonResult::Drift(report) => {
            let mut spec_changes = Vec::new();
            let mut code_changes = Vec::new();
            let mut conflicts = Vec::new();
            
            for change in &report.changes {
                match change {
                    DriftChange::ConceptAdded { name } => {
                        // Concept in code but not in spec
                        spec_changes.push(SpecChange::AddConcept {
                            name: name.clone(),
                            from_code: format!("Extracted from {}", name),
                        });
                    }
                    DriftChange::ConceptRemoved { name } => {
                        // Concept in spec but not in code
                        code_changes.push(CodeChange::AddType {
                            name: name.clone(),
                            from_spec: format!("From Concept {}", name),
                        });
                    }
                    DriftChange::FieldAdded { concept, field } => {
                        spec_changes.push(SpecChange::AddField {
                            concept: concept.clone(),
                            field: field.clone(),
                            type_expr: "TODO".to_string(),
                        });
                    }
                    DriftChange::FieldRemoved { concept, field } => {
                        code_changes.push(CodeChange::AddField {
                            type_name: concept.clone(),
                            field: field.clone(),
                            type_expr: "TODO".to_string(),
                        });
                    }
                    DriftChange::FieldTypeChanged { concept, field, from, to } => {
                        conflicts.push(Conflict {
                            location: format!("{}.{}", concept, field),
                            spec_value: from.clone(),
                            code_value: to.clone(),
                            suggestion: ConflictResolution::Manual,
                        });
                    }
                    DriftChange::BehaviorSignatureChanged { name, diff } => {
                        conflicts.push(Conflict {
                            location: format!("Behavior {}", name),
                            spec_value: "spec signature".to_string(),
                            code_value: "code signature".to_string(),
                            suggestion: ConflictResolution::Manual,
                        });
                    }
                }
            }
            
            let status = if !conflicts.is_empty() {
                SyncStatus::Diverged
            } else if !spec_changes.is_empty() && code_changes.is_empty() {
                SyncStatus::CodeAhead
            } else if spec_changes.is_empty() && !code_changes.is_empty() {
                SyncStatus::SpecAhead
            } else {
                SyncStatus::Diverged
            };
            
            SyncResult {
                status,
                spec_changes,
                code_changes,
                conflicts,
            }
        }
        ComparisonResult::Error(msg) => {
            SyncResult {
                status: SyncStatus::Diverged,
                spec_changes: Vec::new(),
                code_changes: Vec::new(),
                conflicts: vec![Conflict {
                    location: "comparison".to_string(),
                    spec_value: "".to_string(),
                    code_value: msg,
                    suggestion: ConflictResolution::Manual,
                }],
            }
        }
    }
}
```

---

## Phase 5: CLI & Polish (Weeks 17-18)

**File**: `crates/topos-cli/src/main.rs`

```rust
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "topos")]
#[command(about = "Topos specification language toolchain")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Topos project
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    
    /// Validate spec files
    Check {
        #[arg(default_value = "specs")]
        path: PathBuf,
        #[arg(long)]
        strict: bool,
    },
    
    /// Format spec files
    Format {
        #[arg(default_value = "specs")]
        path: PathBuf,
        #[arg(long)]
        check: bool,
    },
    
    /// Show traceability report
    Trace {
        #[arg(default_value = "specs")]
        path: PathBuf,
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
    
    /// Export specs to other formats
    Export {
        path: PathBuf,
        #[arg(long)]
        format: ExportFormat,
        #[arg(long, short)]
        output: PathBuf,
    },
    
    /// Start LSP server
    Lsp,
    
    /// Start MCP server for AI integration
    Mcp {
        #[arg(long, default_value = "stdio")]
        transport: Transport,
    },
    
    /// Detect spec↔code drift
    Drift {
        #[arg(long)]
        spec_path: PathBuf,
        #[arg(long)]
        code_path: PathBuf,
        #[arg(long, default_value = "auto")]
        language: String,
    },
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Html,
    Markdown,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum ExportFormat {
    Json,
    Yaml,
    Markdown,
    Html,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init { path } => commands::init::run(path),
        Commands::Check { path, strict } => commands::check::run(path, strict),
        Commands::Format { path, check } => commands::format::run(path, check),
        Commands::Trace { path, format } => commands::trace::run(path, format),
        Commands::Export { path, format, output } => commands::export::run(path, format, output),
        Commands::Lsp => topos_lsp::run_server().await,
        Commands::Mcp { transport } => commands::mcp::run(transport).await,
        Commands::Drift { spec_path, code_path, language } => {
            commands::drift::run(spec_path, code_path, language)
        }
    }
}
```

---

## Phase 6: VS Code Extension (Weeks 19-20)

**File**: `editors/vscode/package.json`

```json
{
  "name": "topos",
  "displayName": "Topos",
  "description": "Topos specification language support with AI integration",
  "version": "0.1.0",
  "publisher": "topos-lang",
  "engines": { "vscode": "^1.85.0" },
  "categories": ["Programming Languages", "Linters", "Formatters"],
  "activationEvents": [
    "onLanguage:topos",
    "workspaceContains:**/*.tps",
    "workspaceContains:**/*.topos"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [{
      "id": "topos",
      "aliases": ["Topos", "topos"],
      "extensions": [".tps", ".topos"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [{
      "language": "topos",
      "scopeName": "source.topos",
      "path": "./syntaxes/topos.tmLanguage.json"
    }],
    "configuration": {
      "title": "Topos",
      "properties": {
        "topos.server.path": {
          "type": "string",
          "default": "",
          "description": "Path to the Topos LSP server binary"
        },
        "topos.trace.server": {
          "type": "string",
          "enum": ["off", "messages", "verbose"],
          "default": "off"
        },
        "topos.drift.autoCheck": {
          "type": "boolean",
          "default": true,
          "description": "Automatically check for spec↔code drift"
        },
        "topos.drift.codePath": {
          "type": "string",
          "default": "src",
          "description": "Path to source code for drift detection"
        }
      }
    },
    "commands": [
      { "command": "topos.showTraceability", "title": "Topos: Show Traceability Report" },
      { "command": "topos.checkDrift", "title": "Topos: Check Spec↔Code Drift" },
      { "command": "topos.syncSpec", "title": "Topos: Sync Spec with Code" },
      { "command": "topos.completeHole", "title": "Topos: Complete Typed Hole" }
    ],
    "views": {
      "explorer": [{
        "id": "toposTraceability",
        "name": "Topos Traceability",
        "when": "workspaceHasToposFiles"
      }]
    }
  }
}
```

---

## Success Criteria

### Phase 1: ✓ when
- Tree-sitter grammar passes all test cases
- All AST types compile with `#[derive(Facet)]`
- Parser handles error recovery gracefully
- Parse benchmark < 50ms for 1000 lines

### Phase 2: ✓ when
- Salsa database correctly memoizes all queries
- Name resolution works across files
- Hole analysis provides accurate context
- Traceability report is complete

### Phase 3: ✓ when
- LSP hover shows typed hole information (via Peek)
- Go-to-definition works across files
- Find references finds all usages
- Completion suggests valid types

### Phase 4: ✓ when
- MCP server handles V1 tools (validate, summarize, compile_context)
- facet-diff detects structural changes in types/concepts
- One-way drift detection reports meaningful differences

### Phase 5: ✓ when
- CLI validates specs correctly
- Format produces consistent output
- Drift command identifies structural changes

### Phase 6: ✓ when
- VS Code extension installs
- Syntax highlighting complete
- All commands functional

---

## Maintenance Model

*"Why won't this die of entropy?"* — A spec system only works if it stays alive.

### Ownership Model

| Artifact | Owner | Update Trigger |
|----------|-------|----------------|
| Requirements (`REQ-*`) | Product/PM | Feature changes, user feedback |
| Behaviors | Engineering lead | Architecture decisions |
| Tasks | Individual developers | Sprint planning, implementation |
| Evidence | Task assignee | PR completion |

### PR Workflow Integration

```yaml
# .github/workflows/topos-check.yml
on:
  pull_request:
    paths: ['specs/**', 'src/**']

jobs:
  topos:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install topos
      
      # Validate spec syntax and references
      - run: topos check specs/
      
      # Check traceability coverage
      - run: topos trace --format markdown > traceability.md
      
      # Warn if drift detected
      - run: topos drift --spec-path specs/ --code-path src/ || echo "::warning::Drift detected"
```

### Drift Alert Triage

When `topos drift` reports divergence:

1. **Structural drift** (type mismatch, missing field): Usually spec needs update
2. **Anchor drift** (file moved/renamed): Update `file:` paths in tasks
3. **Evidence drift** (PR merged, coverage changed): Run `topos evidence --refresh`

### Recommended Ceremonies

| Cadence | Activity | Command |
|---------|----------|---------|
| Per-PR | Validate spec | `topos check` |
| Per-PR | Check traceability | `topos trace --format short` |
| Weekly | Drift report | `topos drift --report` |
| Sprint | Coverage review | `topos trace --orphans` |

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Adoption friction** | Medium | High | Markdown compatibility, gradual adoption path |
| **Spec rot** | High | High | PR workflow integration, drift alerts, ownership model |
| **Soft constraint sprawl** | Medium | Medium | Lint metric for `[~]` ratio, require hardening tasks |
| **Security leakage via MCP** | Low | Critical | Default-deny sandbox, explicit allowlists, sensitive redaction |
| **Multi-language extraction complexity** | High | Medium | Defer to V2, single language (Rust) first |
| **Foreign block opacity** | Medium | Low | V1: treat as opaque; V2: shallow indexing |
| **Context compiler format churn** | High | Medium | Abstract output layer, track tool format changes |
| **Evidence toil** | High | Medium | V1: optional with warnings; V2: auto-gathering |

---

*This execution plan is designed to be actionable by both human developers and AI coding assistants. Each phase has clear deliverables and leverages the modern Rust ecosystem centered on facet.rs for reflection-based operations.*
