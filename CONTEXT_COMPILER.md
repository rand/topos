# Context Compiler

The Context Compiler transforms Topos specifications into focused AI context, enabling AI agents to work with precisely the information they need for a specific task.

## The Problem

When working on `TASK-17`, an AI agent doesn't need your entire 5000-line specification. It needs:
- The specific requirements `TASK-17` implements
- The concepts those requirements reference
- The behaviors that define the operations
- The aesthetic constraints that apply
- Nothing else

Stuffing everything into context wastes tokens, dilutes focus, and increases hallucination risk.

## The Solution

```bash
topos context TASK-17 --output .cursorrules
```

This compiles a focused context containing only what's relevant to `TASK-17`, formatted for the target AI tool.

## Output Formats

The Context Compiler generates output for modern AI IDE rule systems. Note that these formats evolve—the compiler tracks current conventions.

### Cursor (`.cursor/rules/*.mdc`)

Cursor uses MDC (Markdown Components) files with frontmatter:

```bash
topos context TASK-17 --format cursor
# Creates: .cursor/rules/task-17.mdc
```

Output structure:
```mdc
---
description: Context for TASK-17: Implement payment UI
globs: ["src/components/Payment*.tsx"]
alwaysApply: false
---

# Task Context: TASK-17

## Requirement: REQ-3
As a customer, I want to pay for orders so that they ship.

## Relevant Concepts
...
```

### Cline (`.clinerules/*.md`)

Cline uses a directory of Markdown files:

```bash
topos context TASK-17 --format cline
# Creates: .clinerules/task-17.md
```

### Windsurf (`.windsurf/rules/*.md`)

Windsurf uses Markdown files with activation modes:

```bash
topos context TASK-17 --format windsurf
# Creates: .windsurf/rules/task-17.md
```

### Legacy Single-File Formats

For backwards compatibility with older tool versions:

```bash
# Legacy Cursor (.cursorrules in root)
topos context TASK-17 --format cursor-legacy > .cursorrules

# Legacy Windsurf (.windsurfrules in root)  
topos context TASK-17 --format windsurf-legacy > .windsurfrules

# Legacy Cline (single .clinerules file)
topos context TASK-17 --format cline-legacy > .clinerules
```

### Raw Markdown (any agent)

```bash
topos context TASK-17 --format markdown
```

### Structured JSON (programmatic)

```bash
topos context TASK-17 --format json
```

## Example

Given this specification:

```topos
spec ECommerce

# Requirements

## REQ-1: User Registration
As a visitor, I want to register so that I can make purchases.

## REQ-2: Order Placement  
As a customer, I want to place orders so that I can buy products.

## REQ-3: Payment Processing
As a customer, I want to pay for orders so that they ship.

# Concepts

Concept User:
  field id (`Identifier`)
  field email (`Email`): unique
  field name (`String`)

Concept Order:
  field id (`Identifier`)
  field user (`User`)
  field items (`List` of `OrderItem`)
  field status (`OrderStatus`)

Concept Payment:
  field id (`Identifier`)
  field order (`Order`)
  field method (`PaymentMethod`)
  field status (`PaymentStatus`)

# Behaviors

Behavior create_user:
  Implements REQ-1.
  given: email (`Email`), name (`String`)
  returns: `User` or `ValidationError`

Behavior place_order:
  Implements REQ-2.
  given: user (`User`), items (`List` of `OrderItem`)
  returns: `Order` or `ValidationError`

Behavior process_payment:
  Implements REQ-3.
  given: order (`Order`), method (`PaymentMethod`)
  returns: `Payment` or `PaymentError`

# Aesthetic

Aesthetic CheckoutFlow:
  feel: [~] "Fast, confident, no friction"
  feedback: [~] "Immediate visual confirmation"

# Tasks

## TASK-1: Implement User model [REQ-1]
file: src/models/user.ts
status: done

## TASK-2: Implement Order model [REQ-2]
file: src/models/order.ts
status: done

## TASK-3: Implement Payment model [REQ-3]
file: src/models/payment.ts
depends: TASK-2
status: pending

## TASK-4: Implement payment UI [REQ-3]
file: src/components/PaymentForm.tsx
depends: TASK-3
status: pending
```

Running:

```bash
topos context TASK-4 --format cursor
```

Produces:

```markdown
# Context: TASK-4 - Implement payment UI

## Task
Implement payment UI for REQ-3: Payment Processing.

**File**: src/components/PaymentForm.tsx
**Dependencies**: TASK-3 (Implement Payment model)
**Status**: pending

## Requirement: REQ-3
As a customer, I want to pay for orders so that they ship.

## Relevant Concepts

### Payment
- id: Identifier
- order: Order
- method: PaymentMethod  
- status: PaymentStatus

### Order (referenced)
- id: Identifier
- user: User
- items: List of OrderItem
- status: OrderStatus

### PaymentMethod (enum)
- one of: credit_card, debit_card, paypal, bank_transfer

### PaymentStatus (enum)
- one of: pending, processing, completed, failed

## Relevant Behavior

### process_payment
Implements REQ-3.

**Given**: order (Order), method (PaymentMethod)
**Returns**: Payment or PaymentError

## Aesthetic Constraints

### CheckoutFlow
- **Feel**: Fast, confident, no friction
- **Feedback**: Immediate visual confirmation

## Implementation Notes

This is a UI component. Adhere to:
- The aesthetic constraints above
- The Payment concept structure
- The process_payment behavior contract

The component should call process_payment and handle both success (Payment) and failure (PaymentError) cases.
```

## How It Works

### 1. Task Resolution

Starting from the target task, resolve:
- Direct requirement links (`[REQ-N]`)
- Transitive dependencies (`depends:` chain)
- File context (existing implementation files)

### 2. Requirement Expansion

For each requirement:
- Extract user story and acceptance criteria
- Find implementing behaviors (`Implements REQ-N`)
- Collect EARS clauses (when/shall)

### 3. Concept Collection

From requirements and behaviors, collect:
- Directly referenced concepts
- Transitively referenced concepts (fields that reference other concepts)
- Related enums and type unions

### 4. Aesthetic Filtering

Include aesthetic blocks that:
- Are explicitly linked to the requirement
- Match the task's domain (UI, API, etc.)
- Apply globally to the spec

### 5. Context Assembly

Assemble into target format with:
- Task summary at top
- Requirement context
- Relevant concepts (pruned)
- Relevant behaviors (pruned)
- Aesthetic constraints
- Implementation notes

## Configuration

### `.topos/context.toml`

```toml
[context]
# Maximum context size (tokens, approximate)
max_tokens = 4000

# Include dependency chain depth
dependency_depth = 2

# Include transitive concept references
transitive_concepts = true

# Include aesthetic blocks
include_aesthetics = true

# Default output format
default_format = "cursor"

[formats.cursor]
output_dir = ".cursor/rules"
extension = ".mdc"
include_frontmatter = true

[formats.windsurf]
output_dir = ".windsurf/rules"
extension = ".md"

[formats.cline]
output_dir = ".clinerules"
extension = ".md"
```

### Multi-Tool Strategy

When your project uses multiple AI IDEs (common in teams), use the `--all` flag:

```bash
# Generate for all configured tools
topos context TASK-17 --all

# Creates:
#   .cursor/rules/task-17.mdc
#   .windsurf/rules/task-17.md
#   .clinerules/task-17.md
```

Configure conflict resolution in `.topos/context.toml`:

```toml
[multi_tool]
# Which tools to generate for with --all
enabled = ["cursor", "windsurf", "cline"]

# Strategy when existing rules conflict
# - "overlay": Topos rules take precedence (recommended)
# - "merge": Append Topos rules to existing
# - "skip": Don't overwrite existing rules
conflict_strategy = "overlay"

# Prefix for Topos-generated rules (helps identify source)
rule_prefix = "topos-"
```

### Per-Task Override

```topos
## TASK-17: Complex feature [REQ-5, REQ-6]

context:
  include: `LegacyAdapter`, `MigrationHelper`
  exclude: `InternalDebugTool`
  notes: |
    This task requires special handling for legacy data.
    See migration guide in docs/migration.md.
```

## CLI Reference

```bash
topos context <TASK_ID> [OPTIONS]

Arguments:
  <TASK_ID>  Task identifier (e.g., TASK-17)

Options:
  -f, --format <FORMAT>   Output format [cursor|cline|windsurf|markdown|json]
  -o, --output <FILE>     Output file (default: stdout)
  -d, --depth <N>         Dependency chain depth (default: 2)
  --no-aesthetics         Exclude aesthetic blocks
  --no-transitive         Exclude transitive concept references
  --max-tokens <N>        Maximum approximate token count
  --dry-run               Show what would be included without generating
```

## Integration Examples

### VS Code Task

```json
{
  "label": "Update Cursor Context",
  "type": "shell",
  "command": "topos context ${input:taskId} -o .cursorrules",
  "problemMatcher": []
}
```

### Git Hook (pre-commit)

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Regenerate context for any task with changed files
changed_files=$(git diff --cached --name-only)
for file in $changed_files; do
  task=$(topos trace --file "$file" --format task-id)
  if [ -n "$task" ]; then
    topos context "$task" -o ".context/$task.md"
  fi
done
```

### CI Integration

```yaml
# .github/workflows/context.yml
on:
  pull_request:
    paths: ['**/*.tps', '**/*.topos']

jobs:
  validate-context:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install topos
      - run: |
          for task in $(topos list-tasks --status pending); do
            topos context "$task" --dry-run
          done
```

## Programmatic API

```rust
use topos_context::{ContextCompiler, Format};

let compiler = ContextCompiler::new(&workspace);

let context = compiler
    .task("TASK-17")
    .format(Format::Cursor)
    .max_tokens(4000)
    .include_aesthetics(true)
    .compile()?;

println!("{}", context.render());

// Or get structured data
let data = context.as_structured();
println!("Requirements: {:?}", data.requirements);
println!("Concepts: {:?}", data.concepts);
```

## MCP Tool

The context compiler is also exposed as an MCP tool:

```json
{
  "name": "compile_context",
  "description": "Generate focused AI context for a specific task",
  "inputSchema": {
    "type": "object",
    "properties": {
      "task_id": {
        "type": "string",
        "description": "Task identifier (e.g., TASK-17)"
      },
      "format": {
        "type": "string",
        "enum": ["cursor", "cline", "windsurf", "markdown", "json"],
        "default": "markdown"
      },
      "max_tokens": {
        "type": "integer",
        "default": 4000
      }
    },
    "required": ["task_id"]
  }
}
```

This allows AI agents to request focused context for tasks they're working on.

## Best Practices

### Do

- Run `topos context` before starting work on a task
- Commit `.cursorrules` changes with related code changes
- Use `--dry-run` to preview what will be included
- Configure `max_tokens` based on your model's context window

### Don't

- Manually edit generated context files
- Include context for unrelated tasks
- Skip regeneration after spec changes
- Ignore aesthetic constraints in UI tasks

### Task Hygiene

For best context compilation:

1. **Link requirements explicitly**: `[REQ-1]` not "implements the user story"
2. **Declare dependencies**: `depends: TASK-N` for all prerequisites
3. **Specify files**: `file:` and `tests:` for every task
4. **Scope appropriately**: One task = one cohesive unit of work
