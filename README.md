# Topos

A semantic contract language for human-AI collaboration in software development.

## Vision

Topos serves as a **checkpoint for human verification of AI understanding**. In AI-assisted development, specifications become the point where humans and AI align on intent before code is generated.

```
Forward:  Intent → AI interprets → Spec → Human reviews → AI generates → Code
                                          ↑
                                   "Did you understand me?"
```

The spec captures *what matters* about software in a structured, human-readable format—not for machine verification of correctness, but for **human verification of AI understanding**.

## What Topos Is (and Isn't)

**Topos IS:**
- A structured prose format for capturing software intent
- A checkpoint between natural language and code generation
- A traceability system (requirements → behaviors → tasks → files)
- A drift detection tool (spec vs. code divergence)
- CommonMark-compatible (renders in any Markdown viewer)

**Topos IS NOT (yet):**
- A formal verification system
- A replacement for tests or type systems
- A bidirectional sync engine (anchored extraction is one-way code→spec)

## Core Principles

1. **Readable over formal**: Prose-like syntax reviewable without training
2. **Markdown-compatible**: Valid CommonMark with structured extensions
3. **Incomplete is okay**: Typed holes `[?]` and soft constraints `[~]` for unknowns
4. **One-way first**: Forward flow (spec→code) before reverse extraction
5. **Boundary-level traceability**: Every externally observable behavior and its tests trace to intent (not every internal line)
6. **Evidence-based**: Tasks require concrete evidence (tests, commits, PRs)

## Quick Example

```topos
spec TaskManagement

# Principles

- Test-First: All implementation follows test-driven development
- Simplicity: Prefer simple solutions; complexity requires justification

# Requirements

## REQ-1: Task Creation

As a team member, I want to create tasks so that I can track my work.

when: user submits task creation form with valid title
the system shall: create a new task with status "todo"

acceptance:
  given: user is authenticated
  when: user creates task with title "Fix login bug"
  then: task appears in task list with status "todo"

# Concepts

Concept Task:
  field id (`Identifier`): unique
  field title (`String`): at least 1 character
  field status (`TaskStatus`): default: `todo`

# Tasks

## TASK-1: Implement Task model [REQ-1]

Create the Task domain model with validation.

file: src/models/task.ts
tests: src/models/task.test.ts
evidence:
  pr: #123
  coverage: 94%
status: done
```

## Key Features

### Typed Holes

Explicit tracking of unknowns with type information:

```topos
[?]                                        # Unknown
[? `PaymentMethod` -> `PaymentResult`]     # Typed signature
[?payment_flow : `Payment` -> `Receipt`]   # Named, trackable
[? involving: `Stock`, `Order`]            # Related concepts
```

### Soft Constraints `[~]`

For aesthetic, subjective, or approximate requirements:

```topos
Aesthetic AppStyle:
  palette: [~] "Warm earth tones"
  motion: [~] "Snappy, high easing"
  feel: [~] "Professional but approachable"

Behavior login_animation:
  ensures:
    animation completes in [~] "under 300ms"
    transition feels [~] "smooth"
```

### Evidence-Based Tasks

Tasks require concrete proof of completion:

```topos
## TASK-1: Implement Task model [REQ-1]

file: src/models/task.ts
tests: src/models/task.test.ts
evidence:
  pr: https://github.com/org/repo/pull/123
  commit: abc123f
  coverage: 94%
  benchmark: p99 < 10ms
status: done
```

### Foreign Blocks (TypeSpec, CUE)

Embed best-in-class specs for what they do well:

~~~topos
# API Types

```typespec
model User {
  id: string;
  email: string;
  @minLength(1) name: string;
}
```

# Validation Rules

```cue
#Order: {
  total: number & >0
  items: [...#Item] & len(items) > 0
  status: "pending" | "paid" | "shipped"
}
```
~~~

### Context Compiler

Generate focused AI context from your spec for modern AI IDEs:

```bash
# Generate rules for Cursor (.cursor/rules/*.mdc)
topos context TASK-1 --format cursor

# Generate rules for Windsurf (.windsurf/rules/*.md)
topos context TASK-1 --format windsurf

# Generate rules for Cline (.clinerules/*.md)
topos context TASK-1 --format cline

# Output: focused context with only REQ-1, Concept Task, related behaviors
```

The Context Compiler solves the **"context window bottleneck"**—when working on TASK-17, your AI doesn't need your entire 5000-line spec. It needs precisely the requirements, concepts, and aesthetic constraints relevant to that task.

### Drift Detection

Detect when specs diverge from each other or when code diverges from spec:

```bash
# Compare two spec versions (structural diff)
topos drift spec_v1.tps spec_v2.tps --structural

# Compare with semantic analysis (LLM-powered)
topos drift spec_v1.tps spec_v2.tps

# Output with semantic analysis:
# Drift Report (strategy: hybrid, semantic: available)
# ==================================================
#
# ## Structural Changes
# Found 2 change(s):
#
# ## Requirements
#   ~ REQ-1 (EARS 'when' clause changed)
#
# ## Semantic Analysis
# - **REQ-1** (requirement): 70% aligned ~ minor drift
#     - [high] ConstraintWeakened: Modal verb changed from 'must' to 'should'
#     - [medium] MeaningChanged: Added SSO as alternative authentication method
```

Semantic drift detection uses an LLM to analyze whether prose changes in requirements represent meaningful specification changes or just rewording. See [Configuration](#configuration) for API key setup.

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Reflection** | [facet.rs 0.32](https://facet.rs) | Serialization, diffing, pretty-printing |
| **Incremental** | [Salsa 0.25](https://salsa-rs.github.io/salsa) | Memoized computation |
| **Parsing** | [tree-sitter 0.25](https://tree-sitter.github.io) | Sub-ms incremental parsing |
| **LSP** | [tower-lsp 0.20](https://github.com/ebkalderon/tower-lsp) | Language server |
| **MCP** | [rmcp 0.12](https://github.com/anthropics/rust-mcp-sdk) | AI tool integration |

## Markdown Compatibility

Topos files are **valid CommonMark**. They render correctly in GitHub, VS Code preview, and any Markdown viewer. The structured elements (Concept, Behavior, etc.) are parsed as special blocks but degrade gracefully to readable prose.

This means:
- No new file format to learn for basic use
- PR reviews work with standard diff tools
- Documentation renders without Topos tooling
- Gradual adoption—start with Markdown, add structure as needed

## Roadmap

### V1 ✅ Complete (January 2026)
- ✅ Language spec with CommonMark compatibility
- ✅ tree-sitter grammar with external scanner (indent/dedent, prose)
- ✅ Parser + formatter + validator
- ✅ Typed AST with CST-to-AST conversion
- ✅ Salsa-based incremental analysis with symbol table
- ✅ LSP with diagnostics, hover, go-to-definition, completions
- ✅ CLI: `check`, `format`, `trace`, `context`, `drift`, `gather`, `extract`
- ✅ Traceability reports (JSON, Markdown)
- ✅ MCP tools: `validate_spec`, `summarize_spec`, `compile_context`, `suggest_hole`, `extract_spec`
- ✅ Drift detection (structural comparison)
- ✅ Property-based tests with proptest
- ✅ End-to-end CLI integration tests

### V2 ✅ Complete (January 2026)
- ✅ VS Code extension with syntax highlighting and LSP
- ✅ Polyglot symbol resolution (TypeSpec/CUE foreign blocks)
- ✅ Auto-evidence gathering (`topos gather`)
- ✅ Semantic drift detection with LLM-as-Judge
- ✅ Typed hole suggestions via MCP tool
- ✅ Anchored reverse extraction (`topos extract` for Rust @topos annotations)

### V3 (Research)
- Bidirectional sync with stable IDs and provenance
- Constraint solver integration (Z3)
- Formal verification pathway

## Installation

```bash
# Build from source
git clone https://github.com/rand/topos.git
cd topos
cargo build --release

# Run the CLI
./target/release/topos --help

# Or install locally
cargo install --path crates/topos-cli
```

### VS Code Extension

Install the extension from `editors/vscode/`:

```bash
cd editors/vscode
npm install
npm run package
code --install-extension topos-*.vsix
```

## Configuration

### LLM Features (Optional)

Topos can use LLM providers for enhanced features:

- **Semantic drift detection**: Analyze whether prose changes in specs represent meaningful changes
- **Typed hole suggestions**: Get intelligent suggestions for filling `[?]` placeholders

To enable LLM features, set your Anthropic API key:

```bash
# Option 1: Create a .env file in your project root
echo "ANTHROPIC_API_KEY=sk-ant-api03-..." > .env

# Option 2: Set environment variable directly
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

The `.env` file is automatically loaded by the CLI. Add `.env` to your `.gitignore` to keep your API key secure.

**Without an API key**: LLM features gracefully degrade to structural-only analysis. You'll see a helpful message explaining how to enable semantic analysis:

```
warning: Semantic analysis unavailable, using structural only
  To enable LLM-based semantic comparison:
    1. Create a .env file with: ANTHROPIC_API_KEY=sk-ant-...
    2. Or set the environment variable directly
    3. Or use --structural to skip this warning
```

## Documentation

| Document | Description |
|----------|-------------|
| [Language Specification](LANGUAGE_SPEC.md) | Complete grammar and semantics |
| [Typed Holes](TYPED_HOLES.md) | Progressive specification refinement |
| [Architecture](ARCHITECTURE.md) | System design and threat model |
| [Context Compiler](CONTEXT_COMPILER.md) | AI-focused context generation |
| [Examples](EXAMPLES.md) | Real-world specification examples |
| [Execution Plan](EXECUTION_PLAN.md) | Implementation roadmap |

## Comparison

| Feature | Topos | TypeSpec | CUE | Plain Markdown |
|---------|-------|----------|-----|----------------|
| Human-readable | ✓ | ~ | ~ | ✓ |
| Markdown-compatible | ✓ | ✗ | ✗ | ✓ |
| Structured parsing | ✓ | ✓ | ✓ | ✗ |
| LSP support | ✓ | ✓ | ✓ | ✗ |
| Typed holes | ✓ | ✗ | ✗ | ✗ |
| Soft constraints | ✓ | ✗ | ✗ | ✗ |
| Traceability | ✓ | ✗ | ✗ | ✗ |
| Evidence tracking | ✓ | ✗ | ✗ | ✗ |
| AI context generation | ✓ | ✗ | ✗ | ✗ |
| API/schema generation | Embeds TypeSpec | ✓ | ✓ | ✗ |
| Constraint validation | Embeds CUE | ✗ | ✓ | ✗ |

**Positioning**: Topos is the *spine* that references best-in-class sub-specs. Use TypeSpec for APIs, CUE for constraints, and Topos to tie them together with traceability and AI context.

## File Format

**Extension**: `.tps` or `.topos` (also valid `.md`)

**MIME Type**: `text/topos` (or `text/markdown`)

**Encoding**: UTF-8

## License

MIT
