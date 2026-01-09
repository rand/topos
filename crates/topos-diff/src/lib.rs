//! Structural and semantic diffing for Topos specifications.
//!
//! This crate provides tools for comparing two Topos specifications
//! and generating drift reports that show what has changed.
//!
//! ## Comparison Strategies
//!
//! - **Structural**: Pure AST-based comparison (fast, deterministic)
//! - **Semantic**: LLM-based meaning comparison (understands intent)
//! - **Hybrid**: Structural + semantic for prose content (default)
//!
//! # Example
//!
//! ```
//! use topos_diff::{diff_specs, DiffReport};
//!
//! let old_source = "spec Old\n";
//! let new_source = "spec New\n";
//!
//! let report = diff_specs(old_source, new_source);
//! // report.is_empty() may be true if only the spec name changed
//! ```
//!
//! # Semantic Diffing
//!
//! ```ignore
//! use topos_diff::{semantic_diff, SemanticDiffOptions, ComparisonStrategy};
//!
//! let options = SemanticDiffOptions {
//!     strategy: ComparisonStrategy::Hybrid,
//!     ..Default::default()
//! };
//!
//! // Async version (requires MCP server)
//! let report = semantic_diff(old_source, new_source, options).await?;
//! ```

pub mod semantic;
pub mod strategy;

pub use semantic::{
    semantic_diff, semantic_diff_sync, SemanticDiffOptions, SemanticDiffReport,
    SemanticDiscrepancy, SemanticElementResult,
};
pub use strategy::{ComparisonStrategy, ElementType};

use std::collections::{HashMap, HashSet};

use topos_syntax::{
    Behavior, Concept, Field, Parser, Requirement, SourceFile, Task,
};

/// A report of differences between two specifications.
#[derive(Debug, Clone, Default)]
pub struct DiffReport {
    /// Requirements that were added.
    pub added_requirements: Vec<String>,
    /// Requirements that were removed.
    pub removed_requirements: Vec<String>,
    /// Requirements that were modified.
    pub modified_requirements: Vec<RequirementDiff>,

    /// Concepts that were added.
    pub added_concepts: Vec<String>,
    /// Concepts that were removed.
    pub removed_concepts: Vec<String>,
    /// Concepts that were modified.
    pub modified_concepts: Vec<ConceptDiff>,

    /// Behaviors that were added.
    pub added_behaviors: Vec<String>,
    /// Behaviors that were removed.
    pub removed_behaviors: Vec<String>,
    /// Behaviors that were modified.
    pub modified_behaviors: Vec<BehaviorDiff>,

    /// Tasks that were added.
    pub added_tasks: Vec<String>,
    /// Tasks that were removed.
    pub removed_tasks: Vec<String>,
    /// Tasks that were modified.
    pub modified_tasks: Vec<TaskDiff>,
}

impl DiffReport {
    /// Returns true if there are no differences.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added_requirements.is_empty()
            && self.removed_requirements.is_empty()
            && self.modified_requirements.is_empty()
            && self.added_concepts.is_empty()
            && self.removed_concepts.is_empty()
            && self.modified_concepts.is_empty()
            && self.added_behaviors.is_empty()
            && self.removed_behaviors.is_empty()
            && self.modified_behaviors.is_empty()
            && self.added_tasks.is_empty()
            && self.removed_tasks.is_empty()
            && self.modified_tasks.is_empty()
    }

    /// Total number of changes.
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.added_requirements.len()
            + self.removed_requirements.len()
            + self.modified_requirements.len()
            + self.added_concepts.len()
            + self.removed_concepts.len()
            + self.modified_concepts.len()
            + self.added_behaviors.len()
            + self.removed_behaviors.len()
            + self.modified_behaviors.len()
            + self.added_tasks.len()
            + self.removed_tasks.len()
            + self.modified_tasks.len()
    }

    /// Format the report as human-readable text.
    #[must_use]
    pub fn format_text(&self) -> String {
        let mut out = String::new();

        if self.is_empty() {
            return "No differences found.".to_string();
        }

        out.push_str(&format!("Found {} change(s):\n\n", self.change_count()));

        // Requirements
        if !self.added_requirements.is_empty()
            || !self.removed_requirements.is_empty()
            || !self.modified_requirements.is_empty()
        {
            out.push_str("## Requirements\n\n");
            for id in &self.added_requirements {
                out.push_str(&format!("  + {}\n", id));
            }
            for id in &self.removed_requirements {
                out.push_str(&format!("  - {}\n", id));
            }
            for diff in &self.modified_requirements {
                out.push_str(&format!("  ~ {} ({})\n", diff.id, diff.changes.join(", ")));
            }
            out.push('\n');
        }

        // Concepts
        if !self.added_concepts.is_empty()
            || !self.removed_concepts.is_empty()
            || !self.modified_concepts.is_empty()
        {
            out.push_str("## Concepts\n\n");
            for name in &self.added_concepts {
                out.push_str(&format!("  + {}\n", name));
            }
            for name in &self.removed_concepts {
                out.push_str(&format!("  - {}\n", name));
            }
            for diff in &self.modified_concepts {
                out.push_str(&format!("  ~ {}\n", diff.name));
                for field in &diff.added_fields {
                    out.push_str(&format!("      + field: {}\n", field));
                }
                for field in &diff.removed_fields {
                    out.push_str(&format!("      - field: {}\n", field));
                }
            }
            out.push('\n');
        }

        // Behaviors
        if !self.added_behaviors.is_empty()
            || !self.removed_behaviors.is_empty()
            || !self.modified_behaviors.is_empty()
        {
            out.push_str("## Behaviors\n\n");
            for name in &self.added_behaviors {
                out.push_str(&format!("  + {}\n", name));
            }
            for name in &self.removed_behaviors {
                out.push_str(&format!("  - {}\n", name));
            }
            for diff in &self.modified_behaviors {
                out.push_str(&format!("  ~ {} ({})\n", diff.name, diff.changes.join(", ")));
            }
            out.push('\n');
        }

        // Tasks
        if !self.added_tasks.is_empty()
            || !self.removed_tasks.is_empty()
            || !self.modified_tasks.is_empty()
        {
            out.push_str("## Tasks\n\n");
            for id in &self.added_tasks {
                out.push_str(&format!("  + {}\n", id));
            }
            for id in &self.removed_tasks {
                out.push_str(&format!("  - {}\n", id));
            }
            for diff in &self.modified_tasks {
                out.push_str(&format!("  ~ {} ({})\n", diff.id, diff.changes.join(", ")));
            }
        }

        out
    }

    /// Format the report as JSON.
    #[must_use]
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Difference in a requirement.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RequirementDiff {
    /// The requirement ID.
    pub id: String,
    /// Description of changes.
    pub changes: Vec<String>,
}

/// Difference in a concept.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConceptDiff {
    /// The concept name.
    pub name: String,
    /// Fields that were added.
    pub added_fields: Vec<String>,
    /// Fields that were removed.
    pub removed_fields: Vec<String>,
    /// Fields that were modified.
    pub modified_fields: Vec<String>,
}

/// Difference in a behavior.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BehaviorDiff {
    /// The behavior name.
    pub name: String,
    /// Description of changes.
    pub changes: Vec<String>,
}

/// Difference in a task.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskDiff {
    /// The task ID.
    pub id: String,
    /// Description of changes.
    pub changes: Vec<String>,
}

// Implement Serialize for DiffReport
impl serde::Serialize for DiffReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("DiffReport", 12)?;
        state.serialize_field("added_requirements", &self.added_requirements)?;
        state.serialize_field("removed_requirements", &self.removed_requirements)?;
        state.serialize_field("modified_requirements", &self.modified_requirements)?;
        state.serialize_field("added_concepts", &self.added_concepts)?;
        state.serialize_field("removed_concepts", &self.removed_concepts)?;
        state.serialize_field("modified_concepts", &self.modified_concepts)?;
        state.serialize_field("added_behaviors", &self.added_behaviors)?;
        state.serialize_field("removed_behaviors", &self.removed_behaviors)?;
        state.serialize_field("modified_behaviors", &self.modified_behaviors)?;
        state.serialize_field("added_tasks", &self.added_tasks)?;
        state.serialize_field("removed_tasks", &self.removed_tasks)?;
        state.serialize_field("modified_tasks", &self.modified_tasks)?;
        state.end()
    }
}

/// Compare two specification sources and return a diff report.
///
/// # Errors
///
/// Returns an error message if either source fails to parse.
pub fn diff_specs(old_source: &str, new_source: &str) -> Result<DiffReport, String> {
    let old_file = Parser::parse(old_source).map_err(|e| format!("Failed to parse old spec: {}", e))?;
    let new_file = Parser::parse(new_source).map_err(|e| format!("Failed to parse new spec: {}", e))?;

    Ok(diff_files(&old_file, &new_file))
}

/// Compare two parsed specification files.
#[must_use]
pub fn diff_files(old: &SourceFile, new: &SourceFile) -> DiffReport {
    let mut report = DiffReport::default();

    // Extract items from both files
    let old_reqs = extract_requirements(old);
    let new_reqs = extract_requirements(new);
    let old_concepts = extract_concepts(old);
    let new_concepts = extract_concepts(new);
    let old_behaviors = extract_behaviors(old);
    let new_behaviors = extract_behaviors(new);
    let old_tasks = extract_tasks(old);
    let new_tasks = extract_tasks(new);

    // Diff requirements
    diff_items(
        &old_reqs,
        &new_reqs,
        &mut report.added_requirements,
        &mut report.removed_requirements,
        diff_requirement,
        &mut report.modified_requirements,
    );

    // Diff concepts
    diff_items(
        &old_concepts,
        &new_concepts,
        &mut report.added_concepts,
        &mut report.removed_concepts,
        diff_concept,
        &mut report.modified_concepts,
    );

    // Diff behaviors
    diff_items(
        &old_behaviors,
        &new_behaviors,
        &mut report.added_behaviors,
        &mut report.removed_behaviors,
        diff_behavior,
        &mut report.modified_behaviors,
    );

    // Diff tasks
    diff_items(
        &old_tasks,
        &new_tasks,
        &mut report.added_tasks,
        &mut report.removed_tasks,
        diff_task,
        &mut report.modified_tasks,
    );

    report
}

/// Generic function to diff two maps of items.
fn diff_items<T, D>(
    old: &HashMap<String, &T>,
    new: &HashMap<String, &T>,
    added: &mut Vec<String>,
    removed: &mut Vec<String>,
    diff_fn: impl Fn(&T, &T) -> Option<D>,
    modified: &mut Vec<D>,
) {
    let old_keys: HashSet<_> = old.keys().collect();
    let new_keys: HashSet<_> = new.keys().collect();

    // Added items
    for key in new_keys.difference(&old_keys) {
        added.push((*key).clone());
    }

    // Removed items
    for key in old_keys.difference(&new_keys) {
        removed.push((*key).clone());
    }

    // Modified items
    for key in old_keys.intersection(&new_keys) {
        if let (Some(old_item), Some(new_item)) = (old.get(*key), new.get(*key))
            && let Some(diff) = diff_fn(old_item, new_item) {
                modified.push(diff);
            }
    }
}

/// Extract requirements from a source file.
fn extract_requirements(file: &SourceFile) -> HashMap<String, &Requirement> {
    let mut map = HashMap::new();
    for section in &file.sections {
        for content in &section.contents {
            if let topos_syntax::SectionContent::Requirement(req) = content {
                map.insert(req.id.value.clone(), req);
            }
        }
    }
    map
}

/// Extract concepts from a source file.
fn extract_concepts(file: &SourceFile) -> HashMap<String, &Concept> {
    let mut map = HashMap::new();
    for section in &file.sections {
        for content in &section.contents {
            if let topos_syntax::SectionContent::Concept(concept) = content {
                map.insert(concept.name.value.clone(), concept);
            }
        }
    }
    map
}

/// Extract behaviors from a source file.
fn extract_behaviors(file: &SourceFile) -> HashMap<String, &Behavior> {
    let mut map = HashMap::new();
    for section in &file.sections {
        for content in &section.contents {
            if let topos_syntax::SectionContent::Behavior(behavior) = content {
                map.insert(behavior.name.value.clone(), behavior);
            }
        }
    }
    map
}

/// Extract tasks from a source file.
fn extract_tasks(file: &SourceFile) -> HashMap<String, &Task> {
    let mut map = HashMap::new();
    for section in &file.sections {
        for content in &section.contents {
            if let topos_syntax::SectionContent::Task(task) = content {
                map.insert(task.id.value.clone(), task);
            }
        }
    }
    map
}

/// Compare two requirements and return a diff if they differ.
fn diff_requirement(old: &Requirement, new: &Requirement) -> Option<RequirementDiff> {
    let mut changes = Vec::new();

    if old.title.text != new.title.text {
        changes.push("title changed".to_string());
    }

    if old.ears_clauses.len() != new.ears_clauses.len() {
        changes.push("EARS clauses changed".to_string());
    }

    if old.acceptance.is_some() != new.acceptance.is_some() {
        changes.push("acceptance criteria changed".to_string());
    }

    if changes.is_empty() {
        None
    } else {
        Some(RequirementDiff {
            id: old.id.value.clone(),
            changes,
        })
    }
}

/// Compare two concepts and return a diff if they differ.
fn diff_concept(old: &Concept, new: &Concept) -> Option<ConceptDiff> {
    let old_fields: HashSet<_> = old.fields.iter().map(|f| &f.name.value).collect();
    let new_fields: HashSet<_> = new.fields.iter().map(|f| &f.name.value).collect();

    let added: Vec<_> = new_fields
        .difference(&old_fields)
        .map(|s| (*s).clone())
        .collect();
    let removed: Vec<_> = old_fields
        .difference(&new_fields)
        .map(|s| (*s).clone())
        .collect();

    // Check for modified fields (same name, different type/constraints)
    let mut modified = Vec::new();
    for field_name in old_fields.intersection(&new_fields) {
        let old_field = old.fields.iter().find(|f| &f.name.value == *field_name);
        let new_field = new.fields.iter().find(|f| &f.name.value == *field_name);
        if let (Some(of), Some(nf)) = (old_field, new_field)
            && !fields_equal(of, nf) {
                modified.push((*field_name).clone());
            }
    }

    if added.is_empty() && removed.is_empty() && modified.is_empty() {
        None
    } else {
        Some(ConceptDiff {
            name: old.name.value.clone(),
            added_fields: added,
            removed_fields: removed,
            modified_fields: modified,
        })
    }
}

/// Check if two fields are equal (ignoring spans).
fn fields_equal(a: &Field, b: &Field) -> bool {
    // Compare field names
    if a.name.value != b.name.value {
        return false;
    }

    // Compare types (simplified - just check if both have types or not)
    if a.ty.is_some() != b.ty.is_some() {
        return false;
    }

    // Compare constraint counts
    if a.constraints.len() != b.constraints.len() {
        return false;
    }

    true
}

/// Compare two behaviors and return a diff if they differ.
fn diff_behavior(old: &Behavior, new: &Behavior) -> Option<BehaviorDiff> {
    let mut changes = Vec::new();

    if old.implements.len() != new.implements.len() {
        changes.push("implements changed".to_string());
    }

    if old.given.len() != new.given.len() {
        changes.push("given clauses changed".to_string());
    }

    if old.requires.len() != new.requires.len() {
        changes.push("requires clauses changed".to_string());
    }

    if old.ensures.len() != new.ensures.len() {
        changes.push("ensures clauses changed".to_string());
    }

    if old.returns.is_some() != new.returns.is_some() {
        changes.push("returns changed".to_string());
    }

    if changes.is_empty() {
        None
    } else {
        Some(BehaviorDiff {
            name: old.name.value.clone(),
            changes,
        })
    }
}

/// Compare two tasks and return a diff if they differ.
fn diff_task(old: &Task, new: &Task) -> Option<TaskDiff> {
    let mut changes = Vec::new();

    if old.title.text != new.title.text {
        changes.push("title changed".to_string());
    }

    if old.req_refs.len() != new.req_refs.len() {
        changes.push("requirement references changed".to_string());
    } else {
        let old_refs: HashSet<_> = old.req_refs.iter().map(|r| &r.value).collect();
        let new_refs: HashSet<_> = new.req_refs.iter().map(|r| &r.value).collect();
        if old_refs != new_refs {
            changes.push("requirement references changed".to_string());
        }
    }

    // Check fields
    let old_fields: HashMap<_, _> = old
        .fields
        .iter()
        .map(|f| (format!("{:?}", f.kind), &f.value.text))
        .collect();
    let new_fields: HashMap<_, _> = new
        .fields
        .iter()
        .map(|f| (format!("{:?}", f.kind), &f.value.text))
        .collect();

    for (kind, old_val) in &old_fields {
        if let Some(new_val) = new_fields.get(kind) {
            if old_val != new_val {
                changes.push(format!("{} changed", kind.to_lowercase()));
            }
        } else {
            changes.push(format!("{} removed", kind.to_lowercase()));
        }
    }

    for kind in new_fields.keys() {
        if !old_fields.contains_key(kind) {
            changes.push(format!("{} added", kind.to_lowercase()));
        }
    }

    if changes.is_empty() {
        None
    } else {
        Some(TaskDiff {
            id: old.id.value.clone(),
            changes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_diff() {
        let source = "spec Test\n";
        let report = diff_specs(source, source).unwrap();
        assert!(report.is_empty());
    }

    #[test]
    fn test_diff_report_format() {
        let mut report = DiffReport::default();
        report.added_concepts.push("User".to_string());
        report.removed_requirements.push("REQ-1".to_string());

        let text = report.format_text();
        assert!(text.contains("User"));
        assert!(text.contains("REQ-1"));
    }

    #[test]
    fn test_change_count() {
        let mut report = DiffReport::default();
        assert_eq!(report.change_count(), 0);

        report.added_concepts.push("A".to_string());
        report.removed_concepts.push("B".to_string());
        assert_eq!(report.change_count(), 2);
    }
}
