# Topos for VS Code

Language support for [Topos](https://github.com/rand/topos) - a semantic contract language for human-AI collaboration in software development.

## Features

- **Syntax Highlighting** - Full highlighting for Topos constructs
- **Language Server** - Diagnostics, hover, go-to-definition, completions
- **Snippets** - Quick insertion of concepts, behaviors, tasks, etc.
- **Commands** - Restart server, show traceability report

## Requirements

The `topos` CLI must be installed and available in your PATH:

```bash
# Build from source
git clone https://github.com/rand/topos.git
cd topos
cargo install --path crates/topos-cli
```

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `topos.server.path` | `"topos"` | Path to the topos executable |
| `topos.server.args` | `["lsp"]` | Arguments for the language server |
| `topos.trace.server` | `"off"` | LSP message tracing level |

## Snippets

| Prefix | Description |
|--------|-------------|
| `spec` | New spec file template |
| `req` | Requirement with EARS and BDD |
| `concept` | Domain concept |
| `behavior` | Behavior specification |
| `task` | Task with file references |
| `hole` | Typed hole placeholder |
| `soft` | Soft constraint |

## Commands

- **Topos: Restart Language Server** - Restart the LSP server
- **Topos: Show Traceability Report** - Run `topos trace` on current file

## Development

```bash
cd editors/vscode
npm install
npm run compile
```

To test locally:
1. Open this folder in VS Code
2. Press F5 to launch Extension Development Host
3. Open a `.tps` file

To package:
```bash
npm run package
# Creates topos-0.2.0.vsix
```

## License

MIT
