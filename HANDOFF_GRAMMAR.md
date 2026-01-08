# Handover Document: Core Grammar and Validation Track

## Context
This document describes the state of the "Core Grammar and Validation" track for the Topos project. The goal is to implement a Tree-sitter grammar for the Topos language and a CLI command `topos check` to validate files.

## Current Status
- **Phase 1 (Grammar Definition): COMPLETE**
  - All 5 corpus tests passing
  - Implemented rules: `spec`, `import`, `requirement`, `concept`, `task`, `behavior`, `invariant`, `aesthetic`, `hole`
  - Scanner handles: `INDENT`/`DEDENT`, `NEWLINE`, context-sensitive `PROSE`
  - Key fixes applied:
    - Changed `header` to only match single `#` (not `##`) to avoid conflict with requirements/tasks
    - Added `prec(1, ...)` to requirement/task repeat blocks to prefer structured clauses over section-level prose
    - Scanner stops prose at `[` to allow `task_ref_list` parsing
    - Added `hole` and `hole_content` rules for typed holes

- **Phase 2 (Analysis Integration): COMPLETE**
  - `topos-analysis` crate wired up to `tree-sitter-topos`
  - `check()` function extracts ERROR nodes as diagnostics
  - 5 tests passing

- **Phase 3 (CLI): COMPLETE**
  - `topos check <file>` validates Topos files
  - Reports diagnostics with file:line:col format
  - Exit code 0 on success, 1 on errors

- **Phase 4 (LSP Real Diagnostics): COMPLETE**
  - Removed panic catch_unwind workaround
  - LSP now uses topos_analysis::check() directly
  - Proper severity mapping (Error/Warning/Info)

## Next Steps

1. **Future Enhancements:**
   - Add traceability checks (Task → Requirement references)
   - Implement `topos format` command
   - Add `topos trace` for dependency visualization
   - Add go-to-definition and hover support to LSP

## Repository State
- **Repo:** `topos` (local)
- **Directory:** `/Users/rand/src/topos`
- **Key Files:**
  - `tree-sitter-topos/grammar.js`
  - `tree-sitter-topos/src/scanner.c`
  - `conductor/tracks/core_grammar_20251220/plan.md`

## Testing
- Run `cd tree-sitter-topos && tree-sitter generate && tree-sitter test` to reproduce failures.
- Run `cd tree-sitter-topos && tree-sitter parse debug.tps` to see parse tree for a sample file.
