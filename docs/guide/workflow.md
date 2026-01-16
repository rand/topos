# Topos Development Workflow

This guide describes the spec-first development workflow with Topos.

## The Spec-First Loop

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   1. SPECIFY    →    2. CONTEXT    →    3. IMPLEMENT           │
│   Write spec         Generate AI        Write code              │
│   requirements       context            with guidance           │
│                                                                 │
│        ↑                                      │                 │
│        │                                      ↓                 │
│                                                                 │
│   5. UPDATE     ←    4. VERIFY     ←    Evidence               │
│   Mark tasks         Check tests,        gathered              │
│   complete           coverage                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Step 1: Specify

Start by writing your specification before any code.

### Define Requirements

```topos
## REQ-1: User Login

As a user, I want to log in so I can access my account.

when: user submits valid credentials
the system shall: create an authenticated session

acceptance:
  given: user has verified account
  when: user enters correct email and password
  then: session is created
```

### Model Concepts

```topos
Concept User:
  field id (`UUID`): unique
  field email (`Email`): unique
  field password_hash (`String`)
```

### Define Behaviors

```topos
Behavior login:
  input: `Email`, `Password`
  output: `Result<Session, LoginError>`

  requires:
    user exists and is not locked

  ensures:
    session is created with valid token
```

### Create Tasks

```topos
## TASK-1: Implement login endpoint [REQ-1]

POST /api/auth/login endpoint.

file: src/api/auth.rs
tests: src/api/auth_test.rs
status: pending
```

### Use Typed Holes for Unknowns

Don't let unknowns block you. Mark them explicitly:

```topos
Behavior process_payment:
  input: `Order`
  output: [? `Order` -> `Receipt`]  # Signature TBD

  ensures:
    [?]  # Postconditions TBD
```

## Step 2: Generate Context

When implementing a task, generate focused AI context:

```bash
# Generate context for TASK-1
topos context TASK-1 my-app.tps

# Output includes:
# - The task description
# - Linked requirements (REQ-1)
# - Related concepts (User, Session)
# - Related behaviors (login)
```

### IDE Integration

For AI-powered IDEs, generate rules files:

```bash
# For Cursor
topos context TASK-1 my-app.tps --format cursor > .cursor/rules/task-1.mdc

# For Windsurf
topos context TASK-1 my-app.tps --format windsurf > .windsurf/rules/task-1.md

# For Cline
topos context TASK-1 my-app.tps --format cline > .clinerules/task-1.md
```

This gives your AI assistant precisely the context it needs—no more, no less.

## Step 3: Implement

Write code guided by the spec context.

### Follow the Spec

Your implementation should match:
- **Behaviors**: Function signatures and error types
- **Concepts**: Data structures and field types
- **Requirements**: Acceptance criteria as tests

### Add Anchors (Optional)

For reverse traceability, add `@topos` annotations:

```rust
// @topos(concept="User", req="REQ-1")
pub struct User {
    // @topos(field="id")
    pub id: Uuid,
    // @topos(field="email")
    pub email: String,
}

// @topos(behavior="login", implements="REQ-1")
pub async fn login(email: &str, password: &str) -> Result<Session, LoginError> {
    // ...
}
```

## Step 4: Verify

### Run Tests

Ensure your implementation meets acceptance criteria:

```bash
cargo test
```

### Check Spec Compliance

```bash
# Validate spec syntax
topos check my-app.tps

# View traceability
topos trace my-app.tps
```

### Gather Evidence

Collect evidence from your git history:

```bash
topos gather my-app.tps

# Finds:
# - Commits touching task files
# - PRs linked to tasks
# - Test coverage changes
```

### Check for Drift

Detect divergence between spec and code:

```bash
topos drift my-app.tps --compare src/
```

## Step 5: Update

Mark tasks complete with evidence:

```topos
## TASK-1: Implement login endpoint [REQ-1]

POST /api/auth/login endpoint.

file: src/api/auth.rs
tests: src/api/auth_test.rs
evidence:
  pr: https://github.com/org/repo/pull/42
  commit: abc123f
  coverage: 92%
status: done
```

## Continuous Workflow

### During Development

```bash
# Check spec continuously
topos check my-app.tps

# Generate context for current task
topos context TASK-X my-app.tps
```

### Before Commits

```bash
# Ensure spec is valid
topos check my-app.tps

# Check all tests pass
cargo test

# View traceability gaps
topos trace my-app.tps --format json | jq '.untasked_requirements'
```

### During Code Review

1. Reviewer checks that implementation matches spec
2. Tasks have proper evidence
3. No drift between spec and code

## Tips for Success

### Start Small
- Begin with requirements and tasks only
- Add concepts as you discover domain models
- Add behaviors as you define APIs

### Keep Spec and Code in Sync
- Update spec when requirements change
- Update implementation to match spec
- Use `topos drift` to detect divergence

### Use Typed Holes Liberally
- `[?]` is better than wrong or missing
- Resolve holes as you learn more
- Track holes with names: `[?payment_flow]`

### Evidence Matters
- Tasks aren't done without evidence
- Link PRs, commits, coverage
- Makes verification concrete

### Generate Context Often
- Don't overload AI with full spec
- Generate task-specific context
- Update context as you switch tasks

## MCP Integration

For AI agents, Topos provides MCP tools:

```bash
# Start MCP server
topos mcp
```

Available tools:
- `validate_spec` - Check spec for errors
- `summarize_spec` - Get spec overview
- `compile_context` - Generate task context
- `suggest_hole` - Get suggestions for typed holes
- `extract_spec` - Extract spec from annotated code

These enable AI assistants to work directly with your specifications.
