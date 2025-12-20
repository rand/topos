# Rust Code Style Guide

## 1. Formatting & Conventions
- **Tooling:** Always use `rustfmt` with the default configuration. CI must fail if code is not formatted.
- **Naming:**
  - `SnakeCase` for crates, modules, functions, methods, and variables.
  - `CamelCase` for types (structs, enums, traits, type aliases).
  - `SCREAMING_SNAKE_CASE` for constants and statics.
- **Imports:** Group imports by crate. Use `std`, `external_crate`, `crate` (internal) order.

## 2. Idioms & Best Practices
- **Clippy:** Adhere to `clippy::pedantic` warnings where reasonable. Allow exceptions explicitly with comments explaining why.
- **Error Handling:**
  - Use `Result<T, E>` for recoverable errors.
  - Use `anyhow::Result` for applications/CLI, and specific `thiserror` enums for libraries.
  - Avoid `.unwrap()` and `.expect()` in production code; restrict them to tests or clear invariants (documented with "SAFETY:" comments).
- **Type Safety:** Use the "Newtype" pattern to enforce type safety (e.g., `struct UserId(String)` instead of passing raw strings).

## 3. Testing
- **Unit Tests:** Co-locate unit tests in the same file within a `#[cfg(test)] mod tests { ... }` module.
- **Integration Tests:** Place integration tests in the `tests/` directory.
- **Documentation Tests:** Ensure public examples in documentation (`///`) are valid and compiled.

## 4. Documentation
- **Public API:** All public items (pub) must have documentation comments (`///`).
- **Module Level:** Each module should have a top-level `//!` comment explaining its purpose.

## 5. Unsafe Code
- **Avoidance:** Avoid `unsafe` unless absolutely necessary for FFI or critical performance (proven by benchmarks).
- **Documentation:** Every `unsafe` block must be preceded by a `// SAFETY:` comment explaining the invariant being upheld.
