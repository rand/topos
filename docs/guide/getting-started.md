# Getting Started with Topos

This guide will help you install Topos and write your first specification.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/rand/topos.git
cd topos

# Build the CLI
cargo build --release

# Install to your PATH
cargo install --path crates/topos-cli
```

### Verify Installation

```bash
topos --version
```

## Your First Spec

Create a file named `my-app.tps`:

```topos
spec MyApp

# Principles

- Simplicity: Keep it simple
- Test-First: Write tests before implementation

# Requirements

## REQ-1: User Login

As a user, I want to log in so I can access my account.

when: user submits valid credentials
the system shall: create an authenticated session

acceptance:
  given: user has an account
  when: user enters correct email and password
  then: user is redirected to dashboard

# Concepts

Concept User:
  field id (`UUID`): unique
  field email (`Email`): unique
  field password_hash (`String`)

Concept Session:
  field id (`UUID`): unique
  field user_id (`UUID`)
  field expires_at (`DateTime`)

# Tasks

## TASK-1: Implement User model [REQ-1]

Create the User domain model.

file: src/models/user.rs
tests: src/models/user_test.rs
status: pending
```

## Check Your Spec

Validate your spec for syntax and semantic errors:

```bash
topos check my-app.tps
```

Expected output:
```
✓ No errors in my-app.tps
```

## View Traceability

See how requirements connect to tasks:

```bash
topos trace my-app.tps
```

Output shows which requirements have implementing tasks and which don't.

## IDE Setup

### VS Code

1. Install the extension:
   ```bash
   cd editors/vscode
   npm install
   npm run package
   code --install-extension topos-*.vsix
   ```

2. Open any `.tps` file to get:
   - Syntax highlighting
   - Real-time error checking
   - Hover documentation
   - Go-to-definition
   - Auto-completion

### Other Editors

Topos files are valid Markdown, so any editor with Markdown support will provide basic syntax highlighting. For full language support, use the LSP:

```bash
topos lsp
```

Configure your editor to connect to the Topos LSP server on stdio.

## Next Steps

- [Language Reference](language-reference.md) - Complete syntax guide
- [Workflow Guide](workflow.md) - Spec-first development workflow
- [CLI Reference](cli-reference.md) - All CLI commands
- [Examples](examples/) - Real-world spec examples

## Quick Tips

1. **Start small**: Begin with just requirements and tasks
2. **Use typed holes**: Write `[?]` for things you don't know yet
3. **Link everything**: Connect tasks to requirements with `[REQ-X]`
4. **Check often**: Run `topos check` frequently during development
5. **Generate context**: Use `topos context TASK-X` when implementing
