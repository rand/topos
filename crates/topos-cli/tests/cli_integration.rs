//! End-to-end integration tests for the Topos CLI.
//!
//! These tests exercise the full CLI workflow including:
//! - `topos check` - Validate spec files
//! - `topos trace` - Generate traceability reports
//! - `topos context` - Compile task context
//! - `topos format` - Format spec files
//! - `topos drift` - Compare spec files

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Get a command for the topos binary.
fn topos() -> Command {
    Command::cargo_bin("topos").unwrap()
}

/// Create a temporary spec file with the given content.
fn create_spec_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

// ============================================================================
// Sample spec content for testing
// ============================================================================

const VALID_SPEC: &str = r#"spec TaskManagement

# Requirements

## REQ-AUTH-1: User Authentication
Users must authenticate before accessing the system.
when: user attempts to access protected resource
the system shall: redirect to login page

## REQ-AUTH-2: Session Management
Sessions must expire after inactivity.

# Concepts

Concept User:
  field id ([?])
  field email ([?])
  field passwordHash ([?])

# Tasks

## TASK-AUTH-1: Implement Login [REQ-AUTH-1]
file: src/auth/login.rs
tests: src/auth/login_test.rs
status: pending

## TASK-AUTH-2: Add Session Timeout [REQ-AUTH-1, REQ-AUTH-2]
status: in_progress
"#;

// This spec has a syntax error (unclosed brackets)
const INVALID_SPEC: &str = r#"spec Test

# Requirements

## REQ-1: Bad Requirement [
This has unclosed brackets which is invalid syntax.
"#;

const MODIFIED_SPEC: &str = r#"spec TaskManagement

# Requirements

## REQ-AUTH-1: User Authentication Updated
Users must authenticate with MFA before accessing the system.
when: user attempts to access protected resource
the system shall: redirect to MFA challenge

## REQ-AUTH-3: Password Policy
Passwords must meet complexity requirements.

# Concepts

Concept User:
  field id ([?])
  field email ([?])
  field passwordHash ([?])
  field mfaEnabled ([?])

# Tasks

## TASK-AUTH-1: Implement Login [REQ-AUTH-1]
file: src/auth/login.rs
tests: src/auth/login_test.rs
status: done
"#;

// ============================================================================
// Check command tests
// ============================================================================

#[test]
fn test_check_valid_spec() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "valid.tps", VALID_SPEC);

    topos()
        .arg("check")
        .arg(&spec_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("✓"));
}

#[test]
fn test_check_invalid_spec() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "invalid.tps", INVALID_SPEC);

    topos()
        .arg("check")
        .arg(&spec_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_check_missing_file() {
    topos()
        .arg("check")
        .arg("/nonexistent/path/spec.tps")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read"));
}

// ============================================================================
// Trace command tests
// ============================================================================

#[test]
fn test_trace_text_output() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("trace")
        .arg(&spec_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Traceability Report"))
        .stdout(predicate::str::contains("REQ-AUTH-1"))
        .stdout(predicate::str::contains("TASK-AUTH-1"));
}

#[test]
fn test_trace_json_output() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("trace")
        .arg(&spec_path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"requirements\""))
        .stdout(predicate::str::contains("\"tasks\""));
}

// ============================================================================
// Context command tests
// ============================================================================

#[test]
fn test_context_markdown() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-1")
        .assert()
        .success()
        .stdout(predicate::str::contains("TASK-AUTH-1"))
        .stdout(predicate::str::contains("REQ-AUTH-1"));
}

#[test]
fn test_context_json() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-1")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"task_id\""))
        .stdout(predicate::str::contains("\"requirements\""));
}

#[test]
fn test_context_cursor_format() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-1")
        .arg("--format")
        .arg("cursor")
        .assert()
        .success()
        .stdout(predicate::str::contains("---"))
        .stdout(predicate::str::contains("description:"));
}

#[test]
fn test_context_missing_task() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-NONEXISTENT")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_context_full_flag() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    // --full flag includes concepts in the output
    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-1")
        .arg("--full")
        .assert()
        .success()
        .stdout(predicate::str::contains("Domain Concepts").or(predicate::str::contains("Concepts")))
        .stdout(predicate::str::contains("User"));
}

// ============================================================================
// Format command tests
// ============================================================================

#[test]
fn test_format_check_mode() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    // format --check returns failure if file would be reformatted, success if already formatted
    // We just verify it runs without crashing and produces output
    topos()
        .arg("format")
        .arg("--check")
        .arg(&spec_path)
        .assert()
        .stdout(predicate::str::contains(&spec_path.to_string_lossy().to_string()).or(
            predicate::str::contains("reformatted").or(predicate::str::contains("✓"))
        ));
}

#[test]
fn test_format_writes_file() {
    let dir = TempDir::new().unwrap();
    // Create a spec with inconsistent formatting
    let unformatted = "spec Test\n# Requirements\n## REQ-1: Test\nDescription\n";
    let spec_path = create_spec_file(&dir, "spec.tps", unformatted);

    topos()
        .arg("format")
        .arg(&spec_path)
        .assert()
        .success();

    let formatted = fs::read_to_string(&spec_path).unwrap();
    // Format should have been applied (content may differ)
    assert!(formatted.contains("spec Test"));
    assert!(formatted.contains("REQ-1"));
}

#[test]
fn test_format_no_files() {
    topos()
        .arg("format")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No files specified"));
}

// ============================================================================
// Drift command tests
// ============================================================================

#[test]
fn test_drift_identical_files() {
    let dir = TempDir::new().unwrap();
    let spec1 = create_spec_file(&dir, "spec1.tps", VALID_SPEC);
    let spec2 = create_spec_file(&dir, "spec2.tps", VALID_SPEC);

    topos()
        .arg("drift")
        .arg(&spec1)
        .arg(&spec2)
        .assert()
        .success()
        .stdout(predicate::str::contains("No differences"));
}

#[test]
fn test_drift_different_files() {
    let dir = TempDir::new().unwrap();
    let spec1 = create_spec_file(&dir, "spec1.tps", VALID_SPEC);
    let spec2 = create_spec_file(&dir, "spec2.tps", MODIFIED_SPEC);

    topos()
        .arg("drift")
        .arg(&spec1)
        .arg(&spec2)
        .assert()
        .success()
        // Should show some difference
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_drift_json_output() {
    let dir = TempDir::new().unwrap();
    let spec1 = create_spec_file(&dir, "spec1.tps", VALID_SPEC);
    let spec2 = create_spec_file(&dir, "spec2.tps", MODIFIED_SPEC);

    topos()
        .arg("drift")
        .arg(&spec1)
        .arg(&spec2)
        .arg("--format")
        .arg("json")
        .assert()
        .success();
}

// ============================================================================
// Help and version tests
// ============================================================================

#[test]
fn test_help() {
    topos()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Topos"))
        .stdout(predicate::str::contains("check"))
        .stdout(predicate::str::contains("trace"))
        .stdout(predicate::str::contains("context"));
}

#[test]
fn test_version() {
    topos()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("topos"));
}

// ============================================================================
// End-to-end workflow tests
// ============================================================================

#[test]
fn test_full_workflow() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "project.tps", VALID_SPEC);

    // 1. Check the spec
    topos()
        .arg("check")
        .arg(&spec_path)
        .assert()
        .success();

    // 2. Generate traceability report
    topos()
        .arg("trace")
        .arg(&spec_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Requirements"));

    // 3. Compile context for a task
    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-1")
        .arg("--format")
        .arg("cursor")
        .assert()
        .success();

    // 4. Format the spec (without --check to actually format it)
    topos()
        .arg("format")
        .arg(&spec_path)
        .assert()
        .success();

    // Verify the file was formatted and is still valid
    topos()
        .arg("check")
        .arg(&spec_path)
        .assert()
        .success();
}

#[test]
fn test_multi_task_context() {
    let dir = TempDir::new().unwrap();
    let spec_path = create_spec_file(&dir, "spec.tps", VALID_SPEC);

    // TASK-AUTH-2 references multiple requirements
    topos()
        .arg("context")
        .arg(&spec_path)
        .arg("TASK-AUTH-2")
        .assert()
        .success()
        .stdout(predicate::str::contains("REQ-AUTH-1"))
        .stdout(predicate::str::contains("REQ-AUTH-2"));
}
