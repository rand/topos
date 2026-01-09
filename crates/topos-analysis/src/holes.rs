//! Typed hole extraction and context gathering.
//!
//! This module extracts `[?]` placeholders from Topos specifications
//! and gathers surrounding context to enable LLM-powered suggestions.

use std::sync::Arc;

use topos_syntax::{Behavior, Concept, SectionContent, Span, TypeExpr, TypedHole};

use crate::db::{self, Db};

/// A hole found in the specification with its surrounding context.
#[derive(Debug, Clone, PartialEq)]
pub struct HoleWithContext {
    /// The hole itself.
    pub hole: TypedHole,

    /// Parsed name from hole content (e.g., `payment_flow` from `[?payment_flow : Type]`).
    pub name: Option<String>,

    /// Parsed type constraint from hole content (e.g., `Payment -> Receipt`).
    pub type_hint: Option<String>,

    /// The parent context where this hole appears.
    pub parent: HoleParent,

    /// Related concepts that might inform the suggestion.
    pub related_concepts: Vec<String>,

    /// Adjacent constraints that provide additional context.
    pub adjacent_constraints: Vec<String>,
}

impl HoleWithContext {
    /// Get the span of this hole.
    pub fn span(&self) -> Span {
        self.hole.span
    }

    /// Check if this hole has a type hint.
    pub fn has_type_hint(&self) -> bool {
        self.type_hint.is_some()
    }

    /// Check if this hole has a name.
    pub fn has_name(&self) -> bool {
        self.name.is_some()
    }

    /// Build a prompt context string for LLM suggestions.
    pub fn prompt_context(&self) -> String {
        let mut ctx = String::new();

        // Parent context
        ctx.push_str(&format!("Location: {}\n", self.parent.description()));

        // Name hint
        if let Some(name) = &self.name {
            ctx.push_str(&format!("Suggested name: {}\n", name));
        }

        // Type hint
        if let Some(ty) = &self.type_hint {
            ctx.push_str(&format!("Type constraint: {}\n", ty));
        }

        // Related concepts
        if !self.related_concepts.is_empty() {
            ctx.push_str(&format!(
                "Related concepts: {}\n",
                self.related_concepts.join(", ")
            ));
        }

        // Adjacent constraints
        if !self.adjacent_constraints.is_empty() {
            ctx.push_str("Adjacent constraints:\n");
            for constraint in &self.adjacent_constraints {
                ctx.push_str(&format!("  - {}\n", constraint));
            }
        }

        ctx
    }
}

/// The parent element containing a hole.
#[derive(Debug, Clone, PartialEq)]
pub enum HoleParent {
    /// Hole in a concept field type.
    ConceptField {
        concept_name: String,
        field_name: String,
    },
    /// Hole in a behavior signature.
    BehaviorSignature {
        behavior_name: String,
        position: SignaturePosition,
    },
    /// Hole in a behavior constraint (requires/ensures).
    BehaviorConstraint {
        behavior_name: String,
        constraint_kind: String,
    },
    /// Hole in a returns clause.
    BehaviorReturns { behavior_name: String },
    /// Hole in an unknown location.
    Unknown,
}

impl HoleParent {
    /// Get a human-readable description of the parent.
    pub fn description(&self) -> String {
        match self {
            Self::ConceptField {
                concept_name,
                field_name,
            } => format!("Field '{}' of concept '{}'", field_name, concept_name),
            Self::BehaviorSignature {
                behavior_name,
                position,
            } => format!(
                "{} of behavior '{}'",
                position.description(),
                behavior_name
            ),
            Self::BehaviorConstraint {
                behavior_name,
                constraint_kind,
            } => format!(
                "{} constraint of behavior '{}'",
                constraint_kind, behavior_name
            ),
            Self::BehaviorReturns { behavior_name } => {
                format!("Returns clause of behavior '{}'", behavior_name)
            }
            Self::Unknown => "Unknown location".to_string(),
        }
    }
}

/// Position within a behavior signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePosition {
    /// Input parameter type.
    Input,
    /// Output/return type.
    Output,
}

impl SignaturePosition {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Input => "Input parameter",
            Self::Output => "Output type",
        }
    }
}

/// Collection of holes found in a specification.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HoleCollection {
    /// All holes with their contexts.
    pub holes: Vec<HoleWithContext>,
}

impl HoleCollection {
    /// Check if there are any holes.
    pub fn is_empty(&self) -> bool {
        self.holes.is_empty()
    }

    /// Get the number of holes.
    pub fn len(&self) -> usize {
        self.holes.len()
    }

    /// Find a hole at a specific position (line, column).
    pub fn find_at(&self, line: u32, column: u32) -> Option<&HoleWithContext> {
        self.holes.iter().find(|h| {
            let span = h.span();
            line >= span.start_line
                && line <= span.end_line
                && (line != span.start_line || column >= span.start_col)
                && (line != span.end_line || column <= span.end_col)
        })
    }

    /// Find a hole containing a byte offset.
    pub fn find_at_offset(&self, offset: u32) -> Option<&HoleWithContext> {
        self.holes
            .iter()
            .find(|h| offset >= h.span().start && offset <= h.span().end)
    }

    /// Get holes in a specific concept.
    pub fn in_concept(&self, concept_name: &str) -> Vec<&HoleWithContext> {
        self.holes
            .iter()
            .filter(|h| match &h.parent {
                HoleParent::ConceptField { concept_name: cn, .. } => cn == concept_name,
                _ => false,
            })
            .collect()
    }

    /// Get holes in a specific behavior.
    pub fn in_behavior(&self, behavior_name: &str) -> Vec<&HoleWithContext> {
        self.holes
            .iter()
            .filter(|h| match &h.parent {
                HoleParent::BehaviorSignature { behavior_name: bn, .. }
                | HoleParent::BehaviorConstraint { behavior_name: bn, .. }
                | HoleParent::BehaviorReturns { behavior_name: bn } => bn == behavior_name,
                _ => false,
            })
            .collect()
    }
}

/// Extract all holes from a source file.
#[salsa::tracked]
pub fn extract_holes(db: &dyn Db, file: db::SourceFile) -> Arc<HoleCollection> {
    let ast = db::parse(db, file);
    let mut collection = HoleCollection::default();

    // Collect all concept names for context
    let concept_names: Vec<String> = ast
        .sections
        .iter()
        .flat_map(|s| s.contents.iter())
        .filter_map(|c| match c {
            SectionContent::Concept(concept) => Some(concept.name.value.clone()),
            _ => None,
        })
        .collect();

    // Process each section
    for section in &ast.sections {
        for content in &section.contents {
            match content {
                SectionContent::Concept(concept) => {
                    extract_concept_holes(concept, &concept_names, &mut collection);
                }
                SectionContent::Behavior(behavior) => {
                    extract_behavior_holes(behavior, &concept_names, &mut collection);
                }
                _ => {}
            }
        }
    }

    Arc::new(collection)
}

/// Extract holes from a concept definition.
fn extract_concept_holes(
    concept: &Concept,
    all_concepts: &[String],
    collection: &mut HoleCollection,
) {
    let concept_name = concept.name.value.clone();

    for field in &concept.fields {
        if let Some(TypeExpr::Hole(hole)) = &field.ty {
            let (name, type_hint) = parse_hole_content(&hole.content);

            // Gather adjacent constraints
            let constraints: Vec<String> = field
                .constraints
                .iter()
                .map(|c| format!("{:?}", c))
                .collect();

            // Find related concepts (other fields' types in this concept)
            let related: Vec<String> = concept
                .fields
                .iter()
                .filter(|f| f.name.value != field.name.value)
                .filter_map(|f| match &f.ty {
                    Some(TypeExpr::Reference(r)) => Some(r.name.clone()),
                    _ => None,
                })
                .filter(|n| all_concepts.contains(n))
                .collect();

            collection.holes.push(HoleWithContext {
                hole: hole.clone(),
                name,
                type_hint,
                parent: HoleParent::ConceptField {
                    concept_name: concept_name.clone(),
                    field_name: field.name.value.clone(),
                },
                related_concepts: related,
                adjacent_constraints: constraints,
            });
        }
    }
}

/// Extract holes from a behavior definition.
///
/// Note: Behavior clauses use Prose (plain text), not TypeExpr.
/// Holes in behaviors are detected by scanning prose text for `[?...]` patterns.
fn extract_behavior_holes(
    behavior: &Behavior,
    all_concepts: &[String],
    collection: &mut HoleCollection,
) {
    let behavior_name = behavior.name.value.clone();

    // Scan given clauses for hole patterns in prose
    for (idx, given) in behavior.given.iter().enumerate() {
        if let Some(hole) = extract_hole_from_prose(&given.text, given.span) {
            let (name, type_hint) = parse_hole_content(&hole.content);
            collection.holes.push(HoleWithContext {
                hole,
                name,
                type_hint,
                parent: HoleParent::BehaviorSignature {
                    behavior_name: behavior_name.clone(),
                    position: SignaturePosition::Input,
                },
                related_concepts: find_behavior_concepts(behavior, all_concepts),
                adjacent_constraints: vec![format!("given clause #{}", idx + 1)],
            });
        }
    }

    // Scan returns clause for hole patterns
    if let Some(returns) = &behavior.returns {
        if let Some(hole) = extract_hole_from_prose(&returns.text, returns.span) {
            let (name, type_hint) = parse_hole_content(&hole.content);
            collection.holes.push(HoleWithContext {
                hole,
                name,
                type_hint,
                parent: HoleParent::BehaviorReturns {
                    behavior_name: behavior_name.clone(),
                },
                related_concepts: find_behavior_concepts(behavior, all_concepts),
                adjacent_constraints: Vec::new(),
            });
        }
    }

    // Scan requires clauses for holes
    for (idx, req) in behavior.requires.iter().enumerate() {
        if let Some(hole) = extract_hole_from_prose(&req.text, req.span) {
            let (name, type_hint) = parse_hole_content(&hole.content);
            collection.holes.push(HoleWithContext {
                hole,
                name,
                type_hint,
                parent: HoleParent::BehaviorConstraint {
                    behavior_name: behavior_name.clone(),
                    constraint_kind: "requires".to_string(),
                },
                related_concepts: find_behavior_concepts(behavior, all_concepts),
                adjacent_constraints: vec![format!("requires clause #{}", idx + 1)],
            });
        }
    }

    // Scan ensures clauses for holes
    for (idx, ens) in behavior.ensures.iter().enumerate() {
        if let Some(hole) = extract_hole_from_prose(&ens.text, ens.span) {
            let (name, type_hint) = parse_hole_content(&hole.content);
            collection.holes.push(HoleWithContext {
                hole,
                name,
                type_hint,
                parent: HoleParent::BehaviorConstraint {
                    behavior_name: behavior_name.clone(),
                    constraint_kind: "ensures".to_string(),
                },
                related_concepts: find_behavior_concepts(behavior, all_concepts),
                adjacent_constraints: vec![format!("ensures clause #{}", idx + 1)],
            });
        }
    }
}

/// Extract a hole from prose text if it contains a `[?...]` pattern.
fn extract_hole_from_prose(text: &str, span: Span) -> Option<TypedHole> {
    // Find `[?` pattern
    let start = text.find("[?")?;
    let end = text[start..].find(']')? + start + 1;

    // Extract content between [? and ]
    let content = &text[start + 2..end - 1];
    let content = if content.is_empty() {
        None
    } else {
        Some(content.trim().to_string())
    };

    Some(TypedHole {
        content,
        span, // Use the prose span as approximation
    })
}

/// Find concepts referenced in a behavior by scanning prose text.
fn find_behavior_concepts(behavior: &Behavior, all_concepts: &[String]) -> Vec<String> {
    let mut concepts = Vec::new();

    // Gather all prose text
    let mut all_text = String::new();
    for given in &behavior.given {
        all_text.push_str(&given.text);
        all_text.push(' ');
    }
    if let Some(returns) = &behavior.returns {
        all_text.push_str(&returns.text);
        all_text.push(' ');
    }
    for req in &behavior.requires {
        all_text.push_str(&req.text);
        all_text.push(' ');
    }
    for ens in &behavior.ensures {
        all_text.push_str(&ens.text);
        all_text.push(' ');
    }

    // Check for concept references (backtick-quoted or plain names)
    for concept in all_concepts {
        if all_text.contains(&format!("`{}`", concept)) || all_text.contains(concept) {
            concepts.push(concept.clone());
        }
    }

    // From implements references
    for imp in &behavior.implements {
        if all_concepts.contains(&imp.value) {
            concepts.push(imp.value.clone());
        }
    }

    concepts
}

/// Parse hole content into name and type hint.
///
/// Handles formats like:
/// - `[?]` -> (None, None)
/// - `[? Type]` -> (None, Some("Type"))
/// - `[?name]` -> (Some("name"), None)
/// - `[?name : Type]` -> (Some("name"), Some("Type"))
fn parse_hole_content(content: &Option<String>) -> (Option<String>, Option<String>) {
    let Some(content) = content else {
        return (None, None);
    };

    let content = content.trim();

    // Check for name : type pattern
    if let Some(colon_pos) = content.find(':') {
        let name_part = content[..colon_pos].trim();
        let type_part = content[colon_pos + 1..].trim();

        let name = if name_part.is_empty() {
            None
        } else {
            Some(name_part.to_string())
        };

        let type_hint = if type_part.is_empty() {
            None
        } else {
            Some(type_part.to_string())
        };

        return (name, type_hint);
    }

    // Check if it looks like a type (contains backticks or ->)
    if content.contains('`') || content.contains("->") {
        return (None, Some(content.to_string()));
    }

    // Check if it looks like a name (lowercase, no spaces)
    if content.chars().all(|c| c.is_alphanumeric() || c == '_')
        && content.chars().next().map_or(false, |c| c.is_lowercase())
    {
        return (Some(content.to_string()), None);
    }

    // Default: treat as type hint if non-empty
    if content.is_empty() {
        (None, None)
    } else {
        (None, Some(content.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::AnalysisDatabase;

    #[test]
    fn test_parse_hole_content_empty() {
        let (name, ty) = parse_hole_content(&None);
        assert!(name.is_none());
        assert!(ty.is_none());
    }

    #[test]
    fn test_parse_hole_content_type_only() {
        let (name, ty) = parse_hole_content(&Some("`String` -> `Int`".to_string()));
        assert!(name.is_none());
        assert_eq!(ty, Some("`String` -> `Int`".to_string()));
    }

    #[test]
    fn test_parse_hole_content_name_only() {
        let (name, ty) = parse_hole_content(&Some("payment_flow".to_string()));
        assert_eq!(name, Some("payment_flow".to_string()));
        assert!(ty.is_none());
    }

    #[test]
    fn test_parse_hole_content_name_and_type() {
        let (name, ty) = parse_hole_content(&Some("payment_flow : `Payment` -> `Receipt`".to_string()));
        assert_eq!(name, Some("payment_flow".to_string()));
        assert_eq!(ty, Some("`Payment` -> `Receipt`".to_string()));
    }

    #[test]
    fn test_extract_holes_from_concept() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept Order:
  field id (`String`)
  field status ([?])
  field total ([? `Currency`])
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let holes = extract_holes(&db, file);

        assert_eq!(holes.len(), 2);

        // First hole: status field
        let status_hole = &holes.holes[0];
        assert!(status_hole.name.is_none());
        assert!(status_hole.type_hint.is_none());
        match &status_hole.parent {
            HoleParent::ConceptField { concept_name, field_name } => {
                assert_eq!(concept_name, "Order");
                assert_eq!(field_name, "status");
            }
            _ => panic!("Expected ConceptField parent"),
        }

        // Second hole: total field with type hint
        let total_hole = &holes.holes[1];
        assert!(total_hole.name.is_none());
        assert_eq!(total_hole.type_hint, Some("`Currency`".to_string()));
    }

    #[test]
    fn test_extract_holes_with_related_concepts() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept User:
  field id

Concept Order:
  field owner (`User`)
  field status ([?])
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let holes = extract_holes(&db, file);

        assert_eq!(holes.len(), 1);
        let hole = &holes.holes[0];
        assert!(hole.related_concepts.contains(&"User".to_string()));
    }

    #[test]
    fn test_find_hole_at_offset() {
        let mut db = AnalysisDatabase::new();
        let source = r#"spec Test

# Concepts

Concept Order:
  field status ([?])
"#;
        let file = db.add_file("test.tps".to_string(), source.to_string());
        let holes = extract_holes(&db, file);

        // Find the hole by its offset
        let hole = holes.holes.first().unwrap();
        let found = holes.find_at_offset(hole.span().start);
        assert!(found.is_some());
    }

    #[test]
    fn test_prompt_context() {
        let hole = HoleWithContext {
            hole: TypedHole {
                content: Some("payment : `Money`".to_string()),
                span: Span::default(),
            },
            name: Some("payment".to_string()),
            type_hint: Some("`Money`".to_string()),
            parent: HoleParent::ConceptField {
                concept_name: "Order".to_string(),
                field_name: "total".to_string(),
            },
            related_concepts: vec!["User".to_string(), "Product".to_string()],
            adjacent_constraints: vec!["unique".to_string()],
        };

        let ctx = hole.prompt_context();
        assert!(ctx.contains("Field 'total' of concept 'Order'"));
        assert!(ctx.contains("payment"));
        assert!(ctx.contains("`Money`"));
        assert!(ctx.contains("User"));
        assert!(ctx.contains("unique"));
    }

    #[test]
    fn test_extract_hole_from_prose() {
        let prose = "The input should be [? `Amount`] representing the payment";
        let span = Span::default();
        let hole = extract_hole_from_prose(prose, span);

        assert!(hole.is_some());
        let hole = hole.unwrap();
        assert_eq!(hole.content, Some("`Amount`".to_string()));
    }

    #[test]
    fn test_extract_hole_from_prose_empty() {
        let prose = "The input should be [?] representing the payment";
        let span = Span::default();
        let hole = extract_hole_from_prose(prose, span);

        assert!(hole.is_some());
        let hole = hole.unwrap();
        assert!(hole.content.is_none());
    }

    #[test]
    fn test_no_hole_in_prose() {
        let prose = "The input should be a string representing the payment";
        let span = Span::default();
        let hole = extract_hole_from_prose(prose, span);

        assert!(hole.is_none());
    }
}
