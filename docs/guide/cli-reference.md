# Topos CLI Reference

Complete reference for all Topos CLI commands.

## Installation

```bash
cargo install --path crates/topos-cli
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `check` | Validate a spec file for errors |
| `trace` | Show traceability report |
| `context` | Generate AI context for a task |
| `drift` | Compare two spec versions |
| `format` | Format spec files |
| `gather` | Collect evidence from git history |
| `extract` | Extract spec from annotated Rust code |
| `lsp` | Start the Language Server |
| `mcp` | Start the MCP server |

---

## check

Validate a Topos spec file for syntax and semantic errors.

```bash
topos check <FILE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILE` | Path to the `.tps` file to check |

### Examples

```bash
# Check a single file
topos check my-app.tps

# Check with full path
topos check specs/auth.tps
```

### Output

```
✓ No errors in my-app.tps
```

Or with errors:

```
my-app.tps:15:5: error: Unresolved reference 'REQ-MISSING'
my-app.tps:23:1: warning: Requirement 'REQ-1' has no implementing tasks
```

---

## trace

Show traceability report linking requirements to tasks.

```bash
topos trace [OPTIONS] <FILE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILE` | Path to the `.tps` file |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --format <FORMAT>` | Output format: `text` or `json` | `text` |

### Examples

```bash
# Human-readable report
topos trace my-app.tps

# JSON output for tooling
topos trace my-app.tps --format json

# Pipe to jq for analysis
topos trace my-app.tps -f json | jq '.untasked_requirements'
```

### Output (text)

```
Traceability Report: my-app.tps

Requirements: 5
  - REQ-1: User Login → TASK-1, TASK-2
  - REQ-2: User Logout → TASK-3
  - REQ-3: Password Reset → (no tasks)

Tasks: 3
  - TASK-1: Implement login [REQ-1] (done)
  - TASK-2: Add rate limiting [REQ-1] (pending)
  - TASK-3: Implement logout [REQ-2] (pending)

Coverage: 2/3 requirements have tasks (67%)
```

---

## context

Compile focused AI context for a specific task.

```bash
topos context [OPTIONS] <FILE> <TASK_ID>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILE` | Path to the `.tps` file |
| `TASK_ID` | Task ID (e.g., `TASK-1`) |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --format <FORMAT>` | Output format (see below) | `markdown` |
| `--full` | Include all related concepts and behaviors | off |

### Formats

| Format | Description | Use Case |
|--------|-------------|----------|
| `markdown` | Plain Markdown | General use |
| `cursor` | Cursor `.mdc` format | Cursor IDE |
| `windsurf` | Windsurf rules format | Windsurf IDE |
| `cline` | Cline rules format | Cline extension |
| `json` | JSON output | Tooling integration |

### Examples

```bash
# Generate Markdown context
topos context my-app.tps TASK-1

# Generate Cursor rules
topos context my-app.tps TASK-1 --format cursor > .cursor/rules/task-1.mdc

# Generate full context with all related elements
topos context my-app.tps TASK-1 --full

# JSON for programmatic use
topos context my-app.tps TASK-1 -f json | jq '.requirements'
```

---

## drift

Compare two spec files and show differences.

```bash
topos drift [OPTIONS] <OLD> <NEW>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `OLD` | The original/baseline spec file |
| `NEW` | The new/changed spec file |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-f, --format <FORMAT>` | Output format: `text` or `json` | `text` |
| `--structural` | Use structural comparison only (no LLM) | off |
| `--semantic` | Use semantic (LLM) comparison only | off |

### Examples

```bash
# Compare two versions
topos drift spec-v1.tps spec-v2.tps

# Structural only (fast, no API calls)
topos drift spec-v1.tps spec-v2.tps --structural

# JSON output
topos drift spec-v1.tps spec-v2.tps -f json
```

### Output

```
Drift Report: spec-v1.tps → spec-v2.tps

Added:
  + REQ-4: New requirement
  + Concept Order

Modified:
  ~ REQ-1: Changed acceptance criteria
  ~ Concept User: Added field 'status'

Removed:
  - TASK-OLD: Removed task
```

---

## format

Format Topos spec files for consistent style.

```bash
topos format [OPTIONS] [FILES]...
```

### Arguments

| Argument | Description |
|----------|-------------|
| `FILES` | Files to format (optional, defaults to stdin) |

### Options

| Option | Description |
|--------|-------------|
| `--check` | Check formatting without modifying files |

### Examples

```bash
# Format a single file in place
topos format my-app.tps

# Format multiple files
topos format specs/*.tps

# Check formatting (CI mode)
topos format --check my-app.tps
```

---

## gather

Gather evidence for tasks from git history.

```bash
topos gather [OPTIONS] [PATH] [TASK_ID]
```

### Arguments

| Argument | Description | Default |
|----------|-------------|---------|
| `PATH` | Path to spec file or directory | `.` |
| `TASK_ID` | Specific task ID to gather for | all tasks |

### Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview changes without modifying files |

### Examples

```bash
# Gather evidence for all tasks in current directory
topos gather

# Gather for a specific spec file
topos gather my-app.tps

# Gather for a specific task
topos gather my-app.tps TASK-1

# Preview what would be gathered
topos gather --dry-run
```

### What It Finds

- Commits touching files listed in `file:` fields
- PRs linked to commits
- Test coverage from CI (if available)
- Related commits by message patterns

---

## extract

Extract a Topos spec from Rust source files with `@topos` annotations.

```bash
topos extract [OPTIONS] <PATHS>...
```

### Arguments

| Argument | Description |
|----------|-------------|
| `PATHS` | Paths to Rust files or directories (supports globs) |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-n, --spec-name <NAME>` | Name for the generated spec | `ExtractedSpec` |
| `-o, --output <FILE>` | Output file (stdout if not specified) | stdout |
| `-m, --merge <FILE>` | Merge with an existing spec file | none |

### Examples

```bash
# Extract from a directory
topos extract src/

# Extract from specific files with glob
topos extract "src/**/*.rs"

# Name the spec and save to file
topos extract src/ --spec-name MyApp --output extracted.tps

# Merge with existing spec
topos extract src/ --merge my-app.tps --output updated.tps
```

### Annotation Format

```rust
// @topos(concept="User", req="REQ-1")
pub struct User {
    // @topos(field="id")
    pub id: Uuid,
}

// @topos(behavior="login", implements="REQ-1")
pub fn login() -> Result<Session, Error> { }
```

---

## lsp

Start the Language Server Protocol server.

```bash
topos lsp
```

The LSP server communicates over stdio and provides:

- Real-time diagnostics
- Hover documentation
- Go-to-definition
- Auto-completion
- Code actions

### Editor Configuration

**VS Code**: Use the Topos extension (recommended)

**Neovim**:
```lua
vim.lsp.start({
  name = 'topos',
  cmd = { 'topos', 'lsp' },
  filetypes = { 'topos' },
})
```

**Emacs** (with lsp-mode):
```elisp
(lsp-register-client
  (make-lsp-client
    :new-connection (lsp-stdio-connection '("topos" "lsp"))
    :major-modes '(topos-mode)
    :server-id 'topos-lsp))
```

---

## mcp

Start the Model Context Protocol server for AI integration.

```bash
topos mcp
```

The MCP server provides tools for AI assistants:

| Tool | Description |
|------|-------------|
| `validate_spec` | Validate a spec file |
| `summarize_spec` | Get spec summary |
| `compile_context` | Generate task context |
| `suggest_hole` | Get suggestions for typed holes |
| `extract_spec` | Extract spec from code |

### Claude Desktop Configuration

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "topos": {
      "command": "topos",
      "args": ["mcp"]
    }
  }
}
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (syntax, semantic, or runtime) |
| 2 | Invalid arguments |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TOPOS_LOG` | Log level: `error`, `warn`, `info`, `debug`, `trace` |
| `ANTHROPIC_API_KEY` | API key for semantic drift detection |

## Common Workflows

### CI/CD Validation

```bash
#!/bin/bash
set -e

# Check all specs
for f in specs/*.tps; do
  topos check "$f"
done

# Verify formatting
topos format --check specs/*.tps

# Check traceability coverage
topos trace specs/main.tps -f json | jq -e '.coverage >= 0.8'
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

topos check specs/*.tps || exit 1
topos format --check specs/*.tps || exit 1
```
