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
- A bidirectional sync engine (see [Roadmap](#roadmap) for planned anchored reverse-sync)

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

Detect when code diverges from spec (one-way, best-effort):

```bash
topos drift src/models/task.ts

# Output:
# ⚠ Drift detected in Task model:
#   Spec: field status (`TaskStatus`): default: `todo`
#   Code: status has no default value
```

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Reflection** | [facet.rs](https://facet.rs) | Serialization, diffing, pretty-printing |
| **Incremental** | [Salsa 0.18+](https://salsa-rs.github.io/salsa) | Memoized computation |
| **Parsing** | [tree-sitter 0.25](https://tree-sitter.github.io) | Sub-ms incremental parsing |
| **LSP** | [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server) | Language server |
| **MCP** | [rmcp 0.8](https://github.com/modelcontextprotocol/rust-sdk) | AI tool integration |

## Markdown Compatibility

Topos files are **valid CommonMark**. They render correctly in GitHub, VS Code preview, and any Markdown viewer. The structured elements (Concept, Behavior, etc.) are parsed as special blocks but degrade gracefully to readable prose.

This means:
- No new file format to learn for basic use
- PR reviews work with standard diff tools
- Documentation renders without Topos tooling
- Gradual adoption—start with Markdown, add structure as needed

## Roadmap

### V1 (Current Target)
- ✅ Language spec with CommonMark compatibility
- 🔄 Parser + formatter + validator
- 🔄 LSP with diagnostics, hover, go-to-definition
- 🔄 CLI: `check`, `format`, `trace`, `context`, `drift`
- 🔄 Traceability reports
- 🔄 MCP tools (forward flow only)

### V2 (Future)
- Anchored reverse extraction (code→spec with explicit markers)
- TypeSpec/CUE foreign block validation
- Typed hole suggestions via LLM
- Multi-language code extraction (Rust, TypeScript)

### V3 (Research)
- Bidirectional sync with stable IDs and provenance
- Constraint solver integration (Z3)
- Formal verification pathway

## Installation

```bash
# CLI and LSP server
cargo install topos

# VS Code extension
code --install-extension topos-lang.topos
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
