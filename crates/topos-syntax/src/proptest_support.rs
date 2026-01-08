//! Property-based testing support with arbitrary generators.
//!
//! This module provides proptest strategies for generating arbitrary
//! Topos specifications to test parser/formatter invariants.

use proptest::prelude::*;

use crate::ast::*;
use crate::span::Span;

/// Generate a valid identifier (alphanumeric, starts with letter).
fn identifier_strategy() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9_]{0,15}".prop_map(|s| s)
}

/// Generate a valid requirement ID.
fn req_id_strategy() -> impl Strategy<Value = String> {
    (1..1000u32).prop_map(|n| format!("REQ-{n}"))
}

/// Generate a valid task ID.
fn task_id_strategy() -> impl Strategy<Value = String> {
    (1..1000u32).prop_map(|n| format!("TASK-{n}"))
}

/// Generate simple prose (avoiding special characters that affect parsing).
fn simple_prose_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z ]{1,50}".prop_map(|s| s.trim().to_string())
}

/// Generate an arbitrary Identifier.
pub fn arb_identifier() -> impl Strategy<Value = Identifier> {
    identifier_strategy().prop_map(|value| Identifier {
        value,
        span: Span::dummy(),
    })
}

/// Generate an arbitrary ReqId.
pub fn arb_req_id() -> impl Strategy<Value = ReqId> {
    req_id_strategy().prop_map(|value| ReqId {
        value,
        span: Span::dummy(),
    })
}

/// Generate an arbitrary TaskId.
pub fn arb_task_id() -> impl Strategy<Value = TaskId> {
    task_id_strategy().prop_map(|value| TaskId {
        value,
        span: Span::dummy(),
    })
}

/// Generate arbitrary prose.
pub fn arb_prose() -> impl Strategy<Value = Prose> {
    simple_prose_strategy().prop_map(|text| Prose {
        text,
        span: Span::dummy(),
    })
}

/// Generate an arbitrary Requirement.
pub fn arb_requirement() -> impl Strategy<Value = Requirement> {
    (arb_req_id(), arb_prose()).prop_map(|(id, title)| Requirement {
        id,
        title,
        ears_clauses: vec![],
        acceptance: None,
        prose: vec![],
        span: Span::dummy(),
    })
}

/// Generate an arbitrary Field.
pub fn arb_field() -> impl Strategy<Value = Field> {
    arb_identifier().prop_map(|name| Field {
        name,
        ty: None,
        constraints: vec![],
        span: Span::dummy(),
    })
}

/// Generate an arbitrary Concept.
pub fn arb_concept() -> impl Strategy<Value = Concept> {
    (arb_identifier(), proptest::collection::vec(arb_field(), 0..5)).prop_map(|(name, fields)| {
        Concept {
            name,
            fields,
            prose: vec![],
            span: Span::dummy(),
        }
    })
}

/// Generate an arbitrary Task.
pub fn arb_task() -> impl Strategy<Value = Task> {
    (
        arb_task_id(),
        arb_prose(),
        proptest::collection::vec(arb_req_id(), 0..3),
    )
        .prop_map(|(id, title, req_refs)| Task {
            id,
            title,
            req_refs,
            fields: vec![],
            prose: vec![],
            span: Span::dummy(),
        })
}

/// Generate a simple spec file with basic content.
pub fn arb_simple_spec() -> impl Strategy<Value = String> {
    (
        identifier_strategy(),
        proptest::collection::vec(arb_requirement(), 0..3),
        proptest::collection::vec(arb_concept(), 0..3),
        proptest::collection::vec(arb_task(), 0..3),
    )
        .prop_map(|(name, reqs, concepts, tasks)| {
            let mut out = format!("spec {name}\n\n");

            if !reqs.is_empty() {
                out.push_str("# Requirements\n\n");
                for req in reqs {
                    out.push_str(&format!(
                        "## {}: {}\nDescription for requirement.\n\n",
                        req.id.value,
                        req.title.text.trim()
                    ));
                }
            }

            if !concepts.is_empty() {
                out.push_str("# Concepts\n\n");
                for concept in concepts {
                    out.push_str(&format!("Concept {}:\n", concept.name.value));
                    for field in concept.fields {
                        out.push_str(&format!("  field {}\n", field.name.value));
                    }
                    out.push('\n');
                }
            }

            if !tasks.is_empty() {
                out.push_str("# Tasks\n\n");
                for task in tasks {
                    if task.req_refs.is_empty() {
                        out.push_str(&format!(
                            "## {}: {}\nstatus: pending\n\n",
                            task.id.value,
                            task.title.text.trim()
                        ));
                    } else {
                        let refs: Vec<_> = task.req_refs.iter().map(|r| r.value.as_str()).collect();
                        out.push_str(&format!(
                            "## {}: {} [{}]\nstatus: pending\n\n",
                            task.id.value,
                            task.title.text.trim(),
                            refs.join(", ")
                        ));
                    }
                }
            }

            out
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{format, FormatConfig};
    use crate::Parser;

    proptest! {
        /// Any generated spec should parse without error.
        #[test]
        fn parse_generated_specs(spec in arb_simple_spec()) {
            let result = Parser::parse(&spec);
            prop_assert!(result.is_ok(), "Failed to parse:\n{}\nError: {:?}", spec, result.err());
        }

        /// Formatting should be idempotent: format(format(x)) == format(x).
        #[test]
        fn format_idempotent(spec in arb_simple_spec()) {
            let config = FormatConfig::default();

            // First, parse and format
            if let Ok(ast1) = Parser::parse(&spec) {
                let formatted1 = format(&ast1, &config);

                // Parse the formatted output and format again
                if let Ok(ast2) = Parser::parse(&formatted1) {
                    let formatted2 = format(&ast2, &config);

                    // Second formatting should be identical to first
                    prop_assert_eq!(formatted1, formatted2, "Formatting not idempotent");
                }
            }
        }

        /// Round-trip: parse → format → parse should preserve structure.
        #[test]
        fn round_trip_preserves_structure(spec in arb_simple_spec()) {
            if let Ok(ast1) = Parser::parse(&spec) {
                let config = FormatConfig::default();
                let formatted = format(&ast1, &config);

                if let Ok(ast2) = Parser::parse(&formatted) {
                    // Check that spec name is preserved
                    prop_assert_eq!(
                        ast1.spec.as_ref().map(|s| &s.name.value),
                        ast2.spec.as_ref().map(|s| &s.name.value),
                        "Spec name not preserved"
                    );

                    // Check section count is preserved
                    prop_assert_eq!(
                        ast1.sections.len(),
                        ast2.sections.len(),
                        "Section count not preserved"
                    );
                }
            }
        }

        /// Requirement IDs should survive round-trip.
        #[test]
        fn req_ids_preserved(spec in arb_simple_spec()) {
            if let Ok(ast1) = Parser::parse(&spec) {
                let config = FormatConfig::default();
                let formatted = format(&ast1, &config);

                if let Ok(ast2) = Parser::parse(&formatted) {
                    // Collect all requirement IDs from both
                    let ids1: std::collections::HashSet<_> = ast1
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Requirement(r) = c {
                                Some(r.id.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let ids2: std::collections::HashSet<_> = ast2
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Requirement(r) = c {
                                Some(r.id.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    prop_assert_eq!(ids1, ids2, "Requirement IDs not preserved");
                }
            }
        }

        /// Task IDs should survive round-trip.
        #[test]
        fn task_ids_preserved(spec in arb_simple_spec()) {
            if let Ok(ast1) = Parser::parse(&spec) {
                let config = FormatConfig::default();
                let formatted = format(&ast1, &config);

                if let Ok(ast2) = Parser::parse(&formatted) {
                    // Collect all task IDs from both
                    let ids1: std::collections::HashSet<_> = ast1
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Task(t) = c {
                                Some(t.id.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let ids2: std::collections::HashSet<_> = ast2
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Task(t) = c {
                                Some(t.id.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    prop_assert_eq!(ids1, ids2, "Task IDs not preserved");
                }
            }
        }

        /// Concept names should survive round-trip.
        #[test]
        fn concept_names_preserved(spec in arb_simple_spec()) {
            if let Ok(ast1) = Parser::parse(&spec) {
                let config = FormatConfig::default();
                let formatted = format(&ast1, &config);

                if let Ok(ast2) = Parser::parse(&formatted) {
                    // Collect all concept names from both
                    let names1: std::collections::HashSet<_> = ast1
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Concept(c) = c {
                                Some(c.name.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    let names2: std::collections::HashSet<_> = ast2
                        .sections
                        .iter()
                        .flat_map(|s| &s.contents)
                        .filter_map(|c| {
                            if let SectionContent::Concept(c) = c {
                                Some(c.name.value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();

                    prop_assert_eq!(names1, names2, "Concept names not preserved");
                }
            }
        }
    }
}
