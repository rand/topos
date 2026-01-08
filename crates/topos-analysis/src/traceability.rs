//! Traceability graph for requirements, behaviors, and tasks.
//!
//! This module builds a graph that tracks the relationships:
//! - Requirements → Behaviors (via `implements:` clause)
//! - Requirements → Tasks (via `[REQ-*]` references)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use topos_syntax::{SectionContent, Span};

use crate::db::{self, Db};

/// A node in the traceability graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceNode {
    /// The node identifier (REQ-*, TASK-*, or behavior name).
    pub id: String,
    /// The kind of node.
    pub kind: TraceNodeKind,
    /// Source location.
    pub span: Span,
}

/// Kinds of nodes in the traceability graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraceNodeKind {
    /// A requirement.
    Requirement,
    /// A behavior.
    Behavior,
    /// A task.
    Task,
}

/// The traceability graph.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceabilityGraph {
    /// All nodes in the graph.
    pub nodes: HashMap<String, TraceNode>,

    /// Edges from requirements to behaviors that implement them.
    /// Key: REQ-*, Value: Set of behavior names.
    pub req_to_behaviors: HashMap<String, HashSet<String>>,

    /// Edges from requirements to tasks that reference them.
    /// Key: REQ-*, Value: Set of TASK-*.
    pub req_to_tasks: HashMap<String, HashSet<String>>,

    /// Reverse edges: behavior to requirements it implements.
    /// Key: behavior name, Value: Set of REQ-*.
    pub behavior_to_reqs: HashMap<String, HashSet<String>>,

    /// Reverse edges: task to requirements it references.
    /// Key: TASK-*, Value: Set of REQ-*.
    pub task_to_reqs: HashMap<String, HashSet<String>>,
}

impl TraceabilityGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a requirement node.
    pub fn add_requirement(&mut self, id: String, span: Span) {
        self.nodes.insert(
            id.clone(),
            TraceNode {
                id,
                kind: TraceNodeKind::Requirement,
                span,
            },
        );
    }

    /// Add a behavior node.
    pub fn add_behavior(&mut self, name: String, span: Span) {
        self.nodes.insert(
            name.clone(),
            TraceNode {
                id: name,
                kind: TraceNodeKind::Behavior,
                span,
            },
        );
    }

    /// Add a task node.
    pub fn add_task(&mut self, id: String, span: Span) {
        self.nodes.insert(
            id.clone(),
            TraceNode {
                id,
                kind: TraceNodeKind::Task,
                span,
            },
        );
    }

    /// Link a behavior to a requirement it implements.
    pub fn link_behavior_to_req(&mut self, behavior: &str, req_id: &str) {
        self.req_to_behaviors
            .entry(req_id.to_string())
            .or_default()
            .insert(behavior.to_string());

        self.behavior_to_reqs
            .entry(behavior.to_string())
            .or_default()
            .insert(req_id.to_string());
    }

    /// Link a task to a requirement it references.
    pub fn link_task_to_req(&mut self, task_id: &str, req_id: &str) {
        self.req_to_tasks
            .entry(req_id.to_string())
            .or_default()
            .insert(task_id.to_string());

        self.task_to_reqs
            .entry(task_id.to_string())
            .or_default()
            .insert(req_id.to_string());
    }

    /// Get all behaviors implementing a requirement.
    pub fn behaviors_for_req(&self, req_id: &str) -> impl Iterator<Item = &str> {
        self.req_to_behaviors
            .get(req_id)
            .into_iter()
            .flat_map(|s| s.iter().map(|s| s.as_str()))
    }

    /// Get all tasks referencing a requirement.
    pub fn tasks_for_req(&self, req_id: &str) -> impl Iterator<Item = &str> {
        self.req_to_tasks
            .get(req_id)
            .into_iter()
            .flat_map(|s| s.iter().map(|s| s.as_str()))
    }

    /// Get all requirements implemented by a behavior.
    pub fn reqs_for_behavior(&self, behavior: &str) -> impl Iterator<Item = &str> {
        self.behavior_to_reqs
            .get(behavior)
            .into_iter()
            .flat_map(|s| s.iter().map(|s| s.as_str()))
    }

    /// Get all requirements referenced by a task.
    pub fn reqs_for_task(&self, task_id: &str) -> impl Iterator<Item = &str> {
        self.task_to_reqs
            .get(task_id)
            .into_iter()
            .flat_map(|s| s.iter().map(|s| s.as_str()))
    }

    /// Get requirements with no behaviors implementing them.
    pub fn uncovered_requirements(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().filter_map(|(id, node)| {
            if node.kind == TraceNodeKind::Requirement
                && !self.req_to_behaviors.contains_key(id)
            {
                Some(id.as_str())
            } else {
                None
            }
        })
    }

    /// Get requirements with no tasks referencing them.
    pub fn untasked_requirements(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().filter_map(|(id, node)| {
            if node.kind == TraceNodeKind::Requirement && !self.req_to_tasks.contains_key(id) {
                Some(id.as_str())
            } else {
                None
            }
        })
    }
}

/// Build the traceability graph for a file.
#[salsa::tracked]
pub fn traceability(db: &dyn Db, file: db::SourceFile) -> Arc<TraceabilityGraph> {
    let ast = db::parse(db, file);
    let mut graph = TraceabilityGraph::new();

    // First pass: collect all nodes
    for section in &ast.sections {
        for content in &section.contents {
            match content {
                SectionContent::Requirement(req) => {
                    graph.add_requirement(req.id.value.clone(), req.span);
                }
                SectionContent::Behavior(behavior) => {
                    graph.add_behavior(behavior.name.value.clone(), behavior.span);
                }
                SectionContent::Task(task) => {
                    graph.add_task(task.id.value.clone(), task.span);
                }
                _ => {}
            }
        }
    }

    // Second pass: collect edges
    for section in &ast.sections {
        for content in &section.contents {
            match content {
                SectionContent::Behavior(behavior) => {
                    for req_id in &behavior.implements {
                        graph.link_behavior_to_req(&behavior.name.value, &req_id.value);
                    }
                }
                SectionContent::Task(task) => {
                    for req_id in &task.req_refs {
                        graph.link_task_to_req(&task.id.value, &req_id.value);
                    }
                }
                _ => {}
            }
        }
    }

    Arc::new(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_empty_graph() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_file("test.tps".to_string(), "spec Test\n".to_string());
        let graph = traceability(&db, file);
        assert!(graph.nodes.is_empty());
    }

    #[test]
    fn test_requirement_task_link() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: First Requirement
Description.

# Tasks

## TASK-1: Implement First [REQ-1]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let graph = traceability(&db, file);

        // Check nodes exist
        assert!(graph.nodes.contains_key("REQ-1"));
        assert!(graph.nodes.contains_key("TASK-1"));

        // Check link
        let tasks: Vec<_> = graph.tasks_for_req("REQ-1").collect();
        assert_eq!(tasks, vec!["TASK-1"]);

        let reqs: Vec<_> = graph.reqs_for_task("TASK-1").collect();
        assert_eq!(reqs, vec!["REQ-1"]);
    }

    #[test]
    fn test_uncovered_requirement() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: Covered Requirement
Description.

## REQ-2: Uncovered Requirement
Description.

# Tasks

## TASK-1: Implement First [REQ-1]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let graph = traceability(&db, file);

        // REQ-2 has no behaviors or tasks
        let uncovered: Vec<_> = graph.uncovered_requirements().collect();
        assert!(uncovered.contains(&"REQ-1")); // No behaviors yet
        assert!(uncovered.contains(&"REQ-2"));

        let untasked: Vec<_> = graph.untasked_requirements().collect();
        assert!(untasked.contains(&"REQ-2"));
        assert!(!untasked.contains(&"REQ-1")); // Has task
    }

    #[test]
    fn test_multiple_links() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Requirements

## REQ-1: First
Desc.

## REQ-2: Second
Desc.

# Tasks

## TASK-1: Multi-req [REQ-1, REQ-2]
status: pending
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let graph = traceability(&db, file);

        let reqs: HashSet<_> = graph.reqs_for_task("TASK-1").collect();
        assert!(reqs.contains("REQ-1"));
        assert!(reqs.contains("REQ-2"));
    }
}
