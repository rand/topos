//! Anchor extraction from Rust source files.
//!
//! This module parses `@topos()` annotations from Rust code comments
//! and associates them with their corresponding code elements.
//!
//! # Annotation Format
//!
//! ```rust,ignore
//! // @topos(req="REQ-1", concept="Order")
//! pub struct Order {
//!     // @topos(field="id")
//!     pub id: Uuid,
//! }
//!
//! // @topos(behavior="create_order", implements="REQ-1")
//! pub fn create_order() -> Order { ... }
//! ```

use std::collections::HashMap;
use std::path::Path;

use regex::Regex;
use tree_sitter::{Node, Parser};

/// A parsed `@topos()` annotation from source code.
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// The type of anchor.
    pub kind: AnchorKind,

    /// All key-value attributes from the annotation.
    pub attributes: HashMap<String, String>,

    /// The source file path.
    pub file_path: String,

    /// Line number where the annotation appears (0-indexed).
    pub line: usize,

    /// The associated code element (struct name, fn name, field name, etc.).
    pub code_element: Option<CodeElement>,
}

impl Anchor {
    /// Get an attribute value.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Get the requirement ID if this anchor references one.
    pub fn req_id(&self) -> Option<&str> {
        self.get("req").or_else(|| self.get("implements"))
    }

    /// Get the concept name if specified.
    pub fn concept_name(&self) -> Option<&str> {
        self.get("concept")
    }

    /// Get the behavior name if specified.
    pub fn behavior_name(&self) -> Option<&str> {
        self.get("behavior")
    }

    /// Get the field name if specified.
    pub fn field_name(&self) -> Option<&str> {
        self.get("field")
    }
}

/// The kind of element being annotated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    /// A concept (struct/type definition).
    Concept,
    /// A behavior (function).
    Behavior,
    /// A field within a concept.
    Field,
    /// A requirement reference.
    Requirement,
    /// Unknown/other annotation.
    Unknown,
}

impl AnchorKind {
    /// Determine anchor kind from attributes.
    fn from_attributes(attrs: &HashMap<String, String>) -> Self {
        if attrs.contains_key("concept") {
            Self::Concept
        } else if attrs.contains_key("behavior") {
            Self::Behavior
        } else if attrs.contains_key("field") {
            Self::Field
        } else if attrs.contains_key("req") {
            Self::Requirement
        } else {
            Self::Unknown
        }
    }
}

/// A code element associated with an anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeElement {
    /// The element type.
    pub kind: CodeElementKind,

    /// The element name.
    pub name: String,

    /// The Rust type (for fields and return types).
    pub rust_type: Option<String>,

    /// Visibility (pub, pub(crate), etc.).
    pub visibility: Option<String>,

    /// The full source text of the element.
    pub source: String,

    /// Start line (0-indexed).
    pub start_line: usize,

    /// End line (0-indexed).
    pub end_line: usize,
}

/// Kind of code element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeElementKind {
    /// A struct definition.
    Struct,
    /// An enum definition.
    Enum,
    /// A function definition.
    Function,
    /// A struct field.
    Field,
    /// A type alias.
    TypeAlias,
    /// An impl block.
    Impl,
    /// Unknown element.
    Unknown,
}

/// Collection of anchors extracted from source files.
#[derive(Debug, Clone, Default)]
pub struct AnchorCollection {
    /// All extracted anchors.
    pub anchors: Vec<Anchor>,

    /// Anchors indexed by file path.
    anchors_by_file: HashMap<String, Vec<usize>>,

    /// Concept anchors indexed by name.
    concepts: HashMap<String, usize>,

    /// Behavior anchors indexed by name.
    behaviors: HashMap<String, usize>,
}

impl AnchorCollection {
    /// Create an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an anchor to the collection.
    pub fn add(&mut self, anchor: Anchor) {
        let idx = self.anchors.len();

        // Index by file
        self.anchors_by_file
            .entry(anchor.file_path.clone())
            .or_default()
            .push(idx);

        // Index concepts
        if let Some(name) = anchor.concept_name() {
            self.concepts.insert(name.to_string(), idx);
        }

        // Index behaviors
        if let Some(name) = anchor.behavior_name() {
            self.behaviors.insert(name.to_string(), idx);
        }

        self.anchors.push(anchor);
    }

    /// Get all anchors from a specific file.
    pub fn from_file(&self, path: &str) -> Vec<&Anchor> {
        self.anchors_by_file
            .get(path)
            .map(|indices| indices.iter().map(|&i| &self.anchors[i]).collect())
            .unwrap_or_default()
    }

    /// Get a concept anchor by name.
    pub fn concept(&self, name: &str) -> Option<&Anchor> {
        self.concepts.get(name).map(|&i| &self.anchors[i])
    }

    /// Get a behavior anchor by name.
    pub fn behavior(&self, name: &str) -> Option<&Anchor> {
        self.behaviors.get(name).map(|&i| &self.anchors[i])
    }

    /// Get all concept anchors.
    pub fn concepts(&self) -> impl Iterator<Item = &Anchor> {
        self.anchors
            .iter()
            .filter(|a| a.kind == AnchorKind::Concept)
    }

    /// Get all behavior anchors.
    pub fn behaviors(&self) -> impl Iterator<Item = &Anchor> {
        self.anchors
            .iter()
            .filter(|a| a.kind == AnchorKind::Behavior)
    }

    /// Get all field anchors.
    pub fn fields(&self) -> impl Iterator<Item = &Anchor> {
        self.anchors
            .iter()
            .filter(|a| a.kind == AnchorKind::Field)
    }

    /// Get all requirement anchors.
    pub fn requirements(&self) -> impl Iterator<Item = &Anchor> {
        self.anchors
            .iter()
            .filter(|a| a.kind == AnchorKind::Requirement)
    }

    /// Get all field anchors for a concept.
    pub fn fields_for_concept(&self, concept_name: &str) -> Vec<&Anchor> {
        // Find field anchors that follow the concept anchor in the same file
        let concept = match self.concept(concept_name) {
            Some(c) => c,
            None => return vec![],
        };

        // Find the line of the next concept or behavior in the same file
        let next_boundary = self
            .anchors
            .iter()
            .filter(|a| {
                matches!(a.kind, AnchorKind::Concept | AnchorKind::Behavior)
                    && a.file_path == concept.file_path
                    && a.line > concept.line
            })
            .map(|a| a.line)
            .min()
            .unwrap_or(usize::MAX);

        self.anchors
            .iter()
            .filter(|a| {
                a.kind == AnchorKind::Field
                    && a.file_path == concept.file_path
                    && a.line > concept.line
                    && a.line < next_boundary
            })
            .collect()
    }

    /// Check if collection is empty.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// Get number of anchors.
    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    /// Generate Topos spec from anchors.
    pub fn generate_spec(&self, spec_name: &str) -> String {
        let mut output = format!("spec {}\n\n", spec_name);

        // Collect requirements
        let req_ids: Vec<_> = self
            .anchors
            .iter()
            .filter_map(|a| a.req_id())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !req_ids.is_empty() {
            output.push_str("# Requirements\n\n");
            for req_id in &req_ids {
                output.push_str(&format!("## {}: Extracted requirement\n", req_id));
                output.push_str("Description extracted from code annotations.\n\n");
            }
        }

        // Generate concepts
        let concept_anchors: Vec<_> = self.concepts().collect();
        if !concept_anchors.is_empty() {
            output.push_str("# Concepts\n\n");
            for anchor in concept_anchors {
                if let Some(name) = anchor.concept_name() {
                    output.push_str(&format!("Concept {}:\n", name));

                    // Add fields
                    for field_anchor in self.fields_for_concept(name) {
                        if let Some(field_name) = field_anchor.field_name() {
                            let type_str = field_anchor
                                .code_element
                                .as_ref()
                                .and_then(|e| e.rust_type.as_ref())
                                .map(|t| format!(" (`{}`)", t))
                                .unwrap_or_default();
                            output.push_str(&format!("  field {}{}\n", field_name, type_str));
                        }
                    }
                    output.push('\n');
                }
            }
        }

        // Generate behaviors
        let behavior_anchors: Vec<_> = self.behaviors().collect();
        if !behavior_anchors.is_empty() {
            output.push_str("# Behaviors\n\n");
            for anchor in behavior_anchors {
                if let Some(name) = anchor.behavior_name() {
                    let implements = anchor
                        .req_id()
                        .map(|r| format!(" [{}]", r))
                        .unwrap_or_default();
                    output.push_str(&format!("Behavior {}{}:\n", name, implements));

                    // Add return type if available
                    if let Some(elem) = &anchor.code_element {
                        if let Some(ret_type) = &elem.rust_type {
                            output.push_str(&format!("  returns `{}`\n", ret_type));
                        }
                    }
                    output.push('\n');
                }
            }
        }

        output
    }
}

/// Extract anchors from a Rust source file.
pub fn extract_anchors(source: &str, file_path: &str) -> AnchorCollection {
    let mut collection = AnchorCollection::new();

    // Parse the Rust source
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Failed to set Rust language");

    let Some(tree) = parser.parse(source, None) else {
        return collection;
    };

    // Find all comments and check for @topos annotations
    let annotation_regex =
        Regex::new(r"@topos\s*\(\s*([^)]*)\s*\)").expect("Invalid annotation regex");
    let attr_regex = Regex::new(r#"(\w+)\s*=\s*"([^"]*)""#).expect("Invalid attr regex");

    // Walk the tree to find comments
    let root = tree.root_node();
    let lines: Vec<&str> = source.lines().collect();

    extract_from_node(
        root,
        source,
        &lines,
        file_path,
        &annotation_regex,
        &attr_regex,
        &mut collection,
    );

    collection
}

/// Recursively extract anchors from a syntax tree node.
fn extract_from_node(
    node: Node,
    source: &str,
    lines: &[&str],
    file_path: &str,
    annotation_regex: &Regex,
    attr_regex: &Regex,
    collection: &mut AnchorCollection,
) {
    // Check if this node is a comment
    if node.kind() == "line_comment" || node.kind() == "block_comment" {
        let comment_text = node.utf8_text(source.as_bytes()).unwrap_or("");

        // Check for @topos annotation
        if let Some(captures) = annotation_regex.captures(comment_text) {
            let attrs_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");

            // Parse attributes
            let mut attributes = HashMap::new();
            for cap in attr_regex.captures_iter(attrs_str) {
                if let (Some(key), Some(value)) = (cap.get(1), cap.get(2)) {
                    attributes.insert(key.as_str().to_string(), value.as_str().to_string());
                }
            }

            let kind = AnchorKind::from_attributes(&attributes);
            let line = node.start_position().row;

            // Find the associated code element (the next sibling or parent's next child)
            let code_element = find_associated_element(node, source, lines);

            collection.add(Anchor {
                kind,
                attributes,
                file_path: file_path.to_string(),
                line,
                code_element,
            });
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_from_node(
            child,
            source,
            lines,
            file_path,
            annotation_regex,
            attr_regex,
            collection,
        );
    }
}

/// Find the code element associated with an annotation comment.
fn find_associated_element(comment_node: Node, source: &str, _lines: &[&str]) -> Option<CodeElement> {
    // Look for the next sibling that is a meaningful code element
    let mut sibling = comment_node.next_sibling();

    while let Some(node) = sibling {
        match node.kind() {
            "struct_item" => {
                return Some(extract_struct_element(node, source));
            }
            "enum_item" => {
                return Some(extract_enum_element(node, source));
            }
            "function_item" => {
                return Some(extract_function_element(node, source));
            }
            "field_declaration" => {
                return Some(extract_field_element(node, source));
            }
            "type_item" => {
                return Some(extract_type_alias_element(node, source));
            }
            "line_comment" | "block_comment" => {
                // Skip other comments
                sibling = node.next_sibling();
                continue;
            }
            _ => {
                sibling = node.next_sibling();
                continue;
            }
        }
    }

    // If no sibling found, check if we're inside a struct and this is a field comment
    if let Some(parent) = comment_node.parent() {
        if parent.kind() == "field_declaration_list" {
            // Look for the next field in the parent
            let mut cursor = parent.walk();
            let mut found_comment = false;
            for child in parent.children(&mut cursor) {
                if child.id() == comment_node.id() {
                    found_comment = true;
                    continue;
                }
                if found_comment && child.kind() == "field_declaration" {
                    return Some(extract_field_element(child, source));
                }
            }
        }
    }

    None
}

/// Extract a struct code element.
fn extract_struct_element(node: Node, source: &str) -> CodeElement {
    let name = find_child_by_kind(node, "type_identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("Unknown")
        .to_string();

    let visibility = find_child_by_kind(node, "visibility_modifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let source_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

    CodeElement {
        kind: CodeElementKind::Struct,
        name,
        rust_type: None,
        visibility,
        source: source_text,
        start_line: node.start_position().row,
        end_line: node.end_position().row,
    }
}

/// Extract an enum code element.
fn extract_enum_element(node: Node, source: &str) -> CodeElement {
    let name = find_child_by_kind(node, "type_identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("Unknown")
        .to_string();

    let visibility = find_child_by_kind(node, "visibility_modifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let source_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

    CodeElement {
        kind: CodeElementKind::Enum,
        name,
        rust_type: None,
        visibility,
        source: source_text,
        start_line: node.start_position().row,
        end_line: node.end_position().row,
    }
}

/// Extract a function code element.
fn extract_function_element(node: Node, source: &str) -> CodeElement {
    let name = find_child_by_kind(node, "identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("Unknown")
        .to_string();

    let visibility = find_child_by_kind(node, "visibility_modifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract return type - look for type after -> in the source
    let source_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();
    let return_type = source_text
        .find("->")
        .map(|pos| {
            let after_arrow = &source_text[pos + 2..];
            // Find the end - either { or where
            let end = after_arrow
                .find('{')
                .or_else(|| after_arrow.find("where"))
                .unwrap_or(after_arrow.len());
            after_arrow[..end].trim().to_string()
        })
        .filter(|s| !s.is_empty());

    CodeElement {
        kind: CodeElementKind::Function,
        name,
        rust_type: return_type,
        visibility,
        source: source_text,
        start_line: node.start_position().row,
        end_line: node.end_position().row,
    }
}

/// Extract a field code element.
fn extract_field_element(node: Node, source: &str) -> CodeElement {
    let name = find_child_by_kind(node, "field_identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("Unknown")
        .to_string();

    let visibility = find_child_by_kind(node, "visibility_modifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    // Extract field type
    let field_type = node
        .children(&mut node.walk())
        .find(|n| {
            n.kind().contains("type")
                && n.kind() != "type_identifier"
                && n.kind() != "visibility_modifier"
        })
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let source_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

    CodeElement {
        kind: CodeElementKind::Field,
        name,
        rust_type: field_type,
        visibility,
        source: source_text,
        start_line: node.start_position().row,
        end_line: node.end_position().row,
    }
}

/// Extract a type alias code element.
fn extract_type_alias_element(node: Node, source: &str) -> CodeElement {
    let name = find_child_by_kind(node, "type_identifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .unwrap_or("Unknown")
        .to_string();

    let visibility = find_child_by_kind(node, "visibility_modifier")
        .and_then(|n| n.utf8_text(source.as_bytes()).ok())
        .map(|s| s.to_string());

    let source_text = node.utf8_text(source.as_bytes()).unwrap_or("").to_string();

    CodeElement {
        kind: CodeElementKind::TypeAlias,
        name,
        rust_type: None,
        visibility,
        source: source_text,
        start_line: node.start_position().row,
        end_line: node.end_position().row,
    }
}

/// Find a child node by kind.
fn find_child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|n| n.kind() == kind)
}

/// Extract anchors from multiple files.
pub fn extract_anchors_from_files<P: AsRef<Path>>(paths: &[P]) -> AnchorCollection {
    let mut collection = AnchorCollection::new();

    for path in paths {
        let path_str = path.as_ref().to_string_lossy().to_string();
        if let Ok(source) = std::fs::read_to_string(path.as_ref()) {
            let file_anchors = extract_anchors(&source, &path_str);
            for anchor in file_anchors.anchors {
                collection.add(anchor);
            }
        }
    }

    collection
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_concept_anchor() {
        let source = r#"
// @topos(req="REQ-1", concept="Order")
pub struct Order {
    pub id: u64,
}
"#;
        let collection = extract_anchors(source, "test.rs");

        assert_eq!(collection.len(), 1);
        let anchor = &collection.anchors[0];
        assert_eq!(anchor.kind, AnchorKind::Concept);
        assert_eq!(anchor.concept_name(), Some("Order"));
        assert_eq!(anchor.req_id(), Some("REQ-1"));

        let elem = anchor.code_element.as_ref().unwrap();
        assert_eq!(elem.kind, CodeElementKind::Struct);
        assert_eq!(elem.name, "Order");
    }

    #[test]
    fn test_extract_behavior_anchor() {
        let source = r#"
// @topos(behavior="create_order", implements="REQ-1")
pub fn create_order() -> Order {
    Order { id: 1 }
}
"#;
        let collection = extract_anchors(source, "test.rs");

        assert_eq!(collection.len(), 1);
        let anchor = &collection.anchors[0];
        assert_eq!(anchor.kind, AnchorKind::Behavior);
        assert_eq!(anchor.behavior_name(), Some("create_order"));
        assert_eq!(anchor.req_id(), Some("REQ-1"));

        let elem = anchor.code_element.as_ref().unwrap();
        assert_eq!(elem.kind, CodeElementKind::Function);
        assert_eq!(elem.name, "create_order");
        assert_eq!(elem.rust_type, Some("Order".to_string()));
    }

    #[test]
    fn test_extract_field_anchors() {
        let source = r#"
// @topos(concept="User")
pub struct User {
    // @topos(field="id")
    pub id: u64,
    // @topos(field="name")
    pub name: String,
}
"#;
        let collection = extract_anchors(source, "test.rs");

        assert_eq!(collection.len(), 3);

        let concept = collection.concept("User").unwrap();
        assert_eq!(concept.kind, AnchorKind::Concept);

        let fields: Vec<_> = collection
            .anchors
            .iter()
            .filter(|a| a.kind == AnchorKind::Field)
            .collect();
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn test_generate_spec() {
        let source = r#"
// @topos(req="REQ-1", concept="Order")
pub struct Order {
    // @topos(field="id")
    pub id: u64,
    // @topos(field="status")
    pub status: String,
}

// @topos(behavior="create_order", implements="REQ-1")
pub fn create_order() -> Order {
    Order { id: 1, status: "new".into() }
}
"#;
        let collection = extract_anchors(source, "test.rs");
        let spec = collection.generate_spec("OrderSystem");

        assert!(spec.contains("spec OrderSystem"));
        assert!(spec.contains("Concept Order:"));
        assert!(spec.contains("field id"));
        assert!(spec.contains("field status"));
        assert!(spec.contains("Behavior create_order [REQ-1]:"));
    }

    #[test]
    fn test_parse_attributes() {
        let source = r#"
// @topos(concept="User", req="REQ-AUTH-1", description="User model")
pub struct User {}
"#;
        let collection = extract_anchors(source, "test.rs");

        assert_eq!(collection.len(), 1);
        let anchor = &collection.anchors[0];
        assert_eq!(anchor.get("concept"), Some("User"));
        assert_eq!(anchor.get("req"), Some("REQ-AUTH-1"));
        assert_eq!(anchor.get("description"), Some("User model"));
    }

    #[test]
    fn test_empty_source() {
        let collection = extract_anchors("", "test.rs");
        assert!(collection.is_empty());
    }

    #[test]
    fn test_no_annotations() {
        let source = r#"
pub struct Order {
    pub id: u64,
}

pub fn create_order() -> Order {
    Order { id: 1 }
}
"#;
        let collection = extract_anchors(source, "test.rs");
        assert!(collection.is_empty());
    }

    #[test]
    fn test_block_comment_annotation() {
        let source = r#"
/* @topos(concept="Order") */
pub struct Order {
    pub id: u64,
}
"#;
        let collection = extract_anchors(source, "test.rs");
        assert_eq!(collection.len(), 1);
        assert_eq!(collection.anchors[0].concept_name(), Some("Order"));
    }
}
