//! Parser that converts tree-sitter CST to typed AST.

use thiserror::Error;
use tree_sitter::{Node, Tree};

use crate::ast::*;
use crate::span::Span;

/// Errors that can occur during parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Tree-sitter parsing failed.
    #[error("parsing failed")]
    TreeSitterError,
    /// Unexpected node kind.
    #[error("unexpected node kind: expected {expected}, found {found}")]
    UnexpectedNode {
        expected: &'static str,
        found: String,
    },
    /// Missing required child node.
    #[error("missing required child: {name}")]
    MissingChild { name: &'static str },
}

/// Result type for parsing operations.
pub type ParseResult<T> = Result<T, ParseError>;

/// Parser for converting tree-sitter CST to typed AST.
pub struct Parser<'a> {
    source: &'a str,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source text.
    #[must_use]
    pub fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Parse source into a tree-sitter tree.
    pub fn parse_tree(source: &str) -> ParseResult<Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_topos::language())
            .expect("failed to load topos language");

        parser.parse(source, None).ok_or(ParseError::TreeSitterError)
    }

    /// Parse source text into a typed AST.
    pub fn parse(source: &str) -> ParseResult<SourceFile> {
        let tree = Self::parse_tree(source)?;
        let parser = Parser::new(source);
        parser.parse_source_file(tree.root_node())
    }

    /// Get the text for a node.
    fn text(&self, node: &Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    /// Get the text for a node as String.
    fn text_string(&self, node: &Node) -> String {
        self.text(node).to_string()
    }

    /// Parse a source file.
    fn parse_source_file(&self, node: Node) -> ParseResult<SourceFile> {
        let span = Span::from_node(&node);
        let mut spec = None;
        let mut imports = Vec::new();
        let mut sections = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "spec_def" => {
                    spec = Some(self.parse_spec_decl(child)?);
                }
                "import_def" => {
                    imports.push(self.parse_import(child)?);
                }
                "section" => {
                    sections.push(self.parse_section(child)?);
                }
                "prose" => {
                    prose.push(self.parse_prose(child));
                }
                "comment" | "ERROR" => {}
                _ => {}
            }
        }

        Ok(SourceFile {
            spec,
            imports,
            sections,
            prose,
            span,
        })
    }

    /// Parse a spec declaration.
    fn parse_spec_decl(&self, node: Node) -> ParseResult<SpecDecl> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;
        Ok(SpecDecl { name, span })
    }

    /// Parse an import statement.
    fn parse_import(&self, node: Node) -> ParseResult<Import> {
        let span = Span::from_node(&node);

        // Check if it's a module import (has "as")
        let has_as = node.children(&mut node.walk()).any(|c| c.kind() == "as");

        let kind = if has_as {
            // import "path" as name
            let path = self.find_child(&node, "string").map(|n| self.parse_string_lit(n))?;
            let alias = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;
            ImportKind::Module { path, alias }
        } else {
            // import from "path": Item1, Item2
            let from = self.find_child_opt(&node, "string").map(|n| self.parse_string_lit(n));
            let items = self
                .find_child_opt(&node, "import_list")
                .map(|list| self.parse_import_list(list))
                .unwrap_or_default();
            ImportKind::Items { from, items }
        };

        Ok(Import { kind, span })
    }

    /// Parse an import list.
    fn parse_import_list(&self, node: Node) -> Vec<ImportItem> {
        let mut items = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_item"
                && let Ok(item) = self.parse_import_item(child)
            {
                items.push(item);
            }
        }
        items
    }

    /// Parse an import item.
    fn parse_import_item(&self, node: Node) -> ParseResult<ImportItem> {
        let span = Span::from_node(&node);
        let reference = self.find_child(&node, "reference").map(|n| self.parse_reference(n))?;
        let alias = self.find_child_opt(&node, "identifier").map(|n| self.parse_identifier(n));
        Ok(ImportItem {
            reference,
            alias,
            span,
        })
    }

    /// Parse a section.
    fn parse_section(&self, node: Node) -> ParseResult<Section> {
        let span = Span::from_node(&node);
        let header = if let Some(header_node) = self.find_child_opt(&node, "header") {
            self.find_child_opt(&header_node, "prose")
                .map(|n| self.parse_prose(n))
                .unwrap_or_else(|| Prose::new("", Span::dummy()))
        } else {
            Prose::new("", Span::dummy())
        };

        let mut contents = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(content) = self.parse_section_content(child) {
                contents.push(content);
            }
        }

        Ok(Section {
            header,
            contents,
            span,
        })
    }

    /// Parse section content.
    fn parse_section_content(&self, node: Node) -> Option<SectionContent> {
        match node.kind() {
            "requirement" => self.parse_requirement(node).ok().map(SectionContent::Requirement),
            "concept" => self.parse_concept(node).ok().map(SectionContent::Concept),
            "behavior" => self.parse_behavior(node).ok().map(SectionContent::Behavior),
            "invariant" => self.parse_invariant(node).ok().map(SectionContent::Invariant),
            "task" => self.parse_task(node).ok().map(SectionContent::Task),
            "aesthetic" => self.parse_aesthetic(node).ok().map(SectionContent::Aesthetic),
            "foreign_block" => self.parse_foreign_block(node).ok().map(SectionContent::ForeignBlock),
            "subsection" => self.parse_subsection(node).ok().map(SectionContent::Subsection),
            "prose" => Some(SectionContent::Prose(self.parse_prose(node))),
            _ => None,
        }
    }

    /// Parse a subsection.
    fn parse_subsection(&self, node: Node) -> ParseResult<Subsection> {
        let span = Span::from_node(&node);
        let mut header = Prose::new("", Span::dummy());
        let mut body = Vec::new();

        let mut cursor = node.walk();
        let mut first_prose = true;
        for child in node.children(&mut cursor) {
            if child.kind() == "prose" {
                if first_prose {
                    header = self.parse_prose(child);
                    first_prose = false;
                } else {
                    body.push(self.parse_prose(child));
                }
            }
        }

        Ok(Subsection { header, body, span })
    }

    /// Parse a requirement.
    fn parse_requirement(&self, node: Node) -> ParseResult<Requirement> {
        let span = Span::from_node(&node);

        let id = self
            .find_child(&node, "identifier")
            .map(|n| ReqId {
                value: self.text_string(&n),
                span: Span::from_node(&n),
            })?;

        let mut title = Prose::new("", Span::dummy());
        let mut ears_clauses = Vec::new();
        let mut acceptance = None;
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        let mut first_prose = true;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "prose" => {
                    if first_prose {
                        title = self.parse_prose(child);
                        first_prose = false;
                    } else {
                        prose.push(self.parse_prose(child));
                    }
                }
                "ears_clause" => {
                    if let Ok(clause) = self.parse_ears_clause(child) {
                        ears_clauses.push(clause);
                    }
                }
                "acceptance" => {
                    acceptance = self.parse_acceptance(child).ok();
                }
                _ => {}
            }
        }

        Ok(Requirement {
            id,
            title,
            ears_clauses,
            acceptance,
            prose,
            span,
        })
    }

    /// Parse an EARS clause.
    fn parse_ears_clause(&self, node: Node) -> ParseResult<EarsClause> {
        let span = Span::from_node(&node);
        let mut when = Prose::new("", Span::dummy());
        let mut shall = Prose::new("", Span::dummy());

        let mut cursor = node.walk();
        let proses: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "prose")
            .collect();

        if proses.len() >= 2 {
            when = self.parse_prose(proses[0]);
            shall = self.parse_prose(proses[1]);
        }

        Ok(EarsClause { when, shall, span })
    }

    /// Parse acceptance criteria.
    fn parse_acceptance(&self, node: Node) -> ParseResult<Acceptance> {
        let span = Span::from_node(&node);
        let mut clauses = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "acc_clause"
                && let Ok(clause) = self.parse_acceptance_clause(child)
            {
                clauses.push(clause);
            }
        }

        Ok(Acceptance { clauses, span })
    }

    /// Parse an acceptance clause.
    fn parse_acceptance_clause(&self, node: Node) -> ParseResult<AcceptanceClause> {
        let span = Span::from_node(&node);
        let text = self.text(&node);

        let kind = if text.starts_with("given:") {
            AcceptanceKind::Given
        } else if text.starts_with("when:") {
            AcceptanceKind::When
        } else {
            AcceptanceKind::Then
        };

        let content = self
            .find_child_opt(&node, "prose")
            .map(|n| self.parse_prose(n))
            .unwrap_or_else(|| Prose::new("", Span::dummy()));

        Ok(AcceptanceClause {
            kind,
            content,
            span,
        })
    }

    /// Parse a concept.
    fn parse_concept(&self, node: Node) -> ParseResult<Concept> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let mut fields = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "field" => {
                    if let Ok(field) = self.parse_field(child) {
                        fields.push(field);
                    }
                }
                "prose" => {
                    prose.push(self.parse_prose(child));
                }
                _ => {}
            }
        }

        Ok(Concept {
            name,
            fields,
            prose,
            span,
        })
    }

    /// Parse a field.
    fn parse_field(&self, node: Node) -> ParseResult<Field> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let ty = self.find_child_opt(&node, "type_expr").and_then(|n| self.parse_type_expr(n).ok());

        let mut constraints = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "constraint"
                && let Ok(c) = self.parse_constraint(child)
            {
                constraints.push(c);
            }
        }

        Ok(Field {
            name,
            ty,
            constraints,
            span,
        })
    }

    /// Parse a type expression.
    fn parse_type_expr(&self, node: Node) -> ParseResult<TypeExpr> {
        let span = Span::from_node(&node);
        let text = self.text(&node);

        // Check for hole
        if let Some(hole_node) = self.find_child_opt(&node, "hole") {
            return Ok(TypeExpr::Hole(self.parse_hole(hole_node)));
        }

        // Check for reference
        if let Some(ref_node) = self.find_child_opt(&node, "reference") {
            // Check if there's a second reference (applied type)
            let refs: Vec<_> = node
                .children(&mut node.walk())
                .filter(|c| c.kind() == "reference")
                .collect();

            if refs.len() >= 2 {
                return Ok(TypeExpr::Applied {
                    base: self.parse_reference(refs[0]),
                    arg: self.parse_reference(refs[1]),
                    span,
                });
            }

            // Check for List/Optional keywords
            if text.starts_with("List") {
                return Ok(TypeExpr::List {
                    element: self.parse_reference(ref_node),
                    span,
                });
            }
            if text.starts_with("Optional") {
                return Ok(TypeExpr::Optional {
                    inner: self.parse_reference(ref_node),
                    span,
                });
            }

            return Ok(TypeExpr::Reference(self.parse_reference(ref_node)));
        }

        // Check for one of
        if let Some(variant_list) = self.find_child_opt(&node, "variant_list") {
            let variants = self.parse_variant_list(variant_list);
            return Ok(TypeExpr::OneOf { variants, span });
        }

        Err(ParseError::UnexpectedNode {
            expected: "type_expr",
            found: node.kind().to_string(),
        })
    }

    /// Parse a typed hole.
    fn parse_hole(&self, node: Node) -> TypedHole {
        let span = Span::from_node(&node);
        let content = self
            .find_child_opt(&node, "hole_content")
            .map(|n| self.text_string(&n));
        TypedHole { content, span }
    }

    /// Parse a variant list.
    fn parse_variant_list(&self, node: Node) -> Vec<Identifier> {
        let mut variants = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                variants.push(self.parse_identifier(child));
            }
        }
        variants
    }

    /// Parse a constraint.
    fn parse_constraint(&self, node: Node) -> ParseResult<Constraint> {
        let span = Span::from_node(&node);
        let text = self.text(&node);

        if text == "unique" {
            return Ok(Constraint::Unique { span });
        }

        if text.starts_with("default:") {
            let value = self
                .find_child_opt(&node, "prose")
                .map(|n| self.parse_prose(n))
                .unwrap_or_else(|| Prose::new("", Span::dummy()));
            return Ok(Constraint::Default { value, span });
        }

        if text.starts_with("derived:") {
            let expr = self
                .find_child_opt(&node, "prose")
                .map(|n| self.parse_prose(n))
                .unwrap_or_else(|| Prose::new("", Span::dummy()));
            return Ok(Constraint::Derived { expr, span });
        }

        if text.starts_with("invariant:") {
            let predicate = self
                .find_child_opt(&node, "prose")
                .map(|n| self.parse_prose(n))
                .unwrap_or_else(|| Prose::new("", Span::dummy()));
            return Ok(Constraint::Invariant { predicate, span });
        }

        if text.starts_with("at least") {
            let count = self
                .find_child_opt(&node, "number")
                .and_then(|n| self.text(&n).parse().ok())
                .unwrap_or(0);
            let unit = self.find_child_opt(&node, "identifier").map(|n| self.parse_identifier(n));
            return Ok(Constraint::AtLeast { count, unit, span });
        }

        Err(ParseError::UnexpectedNode {
            expected: "constraint",
            found: text.to_string(),
        })
    }

    /// Parse a behavior.
    fn parse_behavior(&self, node: Node) -> ParseResult<Behavior> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let mut implements = Vec::new();
        let mut given = Vec::new();
        let mut returns = None;
        let mut requires = Vec::new();
        let mut ensures = Vec::new();
        let mut ears_clauses = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "implements_clause" => {
                    implements.extend(self.parse_implements_clause(child));
                }
                "behavior_body" => {
                    self.parse_behavior_body(
                        child,
                        &mut given,
                        &mut returns,
                        &mut requires,
                        &mut ensures,
                        &mut ears_clauses,
                    );
                }
                "ears_clause" => {
                    if let Ok(clause) = self.parse_ears_clause(child) {
                        ears_clauses.push(clause);
                    }
                }
                "prose" => {
                    prose.push(self.parse_prose(child));
                }
                _ => {}
            }
        }

        Ok(Behavior {
            name,
            implements,
            given,
            returns,
            requires,
            ensures,
            ears_clauses,
            prose,
            span,
        })
    }

    /// Parse implements clause.
    fn parse_implements_clause(&self, node: Node) -> Vec<ReqId> {
        let mut ids = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let text = self.text(&child);
                if text.starts_with("REQ-") {
                    ids.push(ReqId {
                        value: text.to_string(),
                        span: Span::from_node(&child),
                    });
                }
            }
        }
        ids
    }

    /// Parse behavior body content.
    fn parse_behavior_body(
        &self,
        node: Node,
        given: &mut Vec<Prose>,
        returns: &mut Option<Prose>,
        requires: &mut Vec<Prose>,
        ensures: &mut Vec<Prose>,
        ears_clauses: &mut Vec<EarsClause>,
    ) {
        let text = self.text(&node);
        let prose_node = self.find_child_opt(&node, "prose");

        if text.starts_with("given:") {
            if let Some(p) = prose_node {
                given.push(self.parse_prose(p));
            }
        } else if text.starts_with("returns:") {
            if let Some(p) = prose_node {
                *returns = Some(self.parse_prose(p));
            }
        } else if text.starts_with("requires:") {
            if let Some(p) = prose_node {
                requires.push(self.parse_prose(p));
            }
        } else if text.starts_with("ensures:") {
            if let Some(p) = prose_node {
                ensures.push(self.parse_prose(p));
            }
        } else if let Some(ears) = self.find_child_opt(&node, "ears_clause")
            && let Ok(clause) = self.parse_ears_clause(ears)
        {
            ears_clauses.push(clause);
        }
    }

    /// Parse an invariant.
    fn parse_invariant(&self, node: Node) -> ParseResult<Invariant> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let mut quantifiers = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "quantifier" => {
                    if let Ok(q) = self.parse_quantifier(child) {
                        quantifiers.push(q);
                    }
                }
                "prose" => {
                    prose.push(self.parse_prose(child));
                }
                _ => {}
            }
        }

        Ok(Invariant {
            name,
            quantifiers,
            prose,
            span,
        })
    }

    /// Parse a quantifier.
    fn parse_quantifier(&self, node: Node) -> ParseResult<Quantifier> {
        let span = Span::from_node(&node);
        let var = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;
        let collection = self.find_child(&node, "reference").map(|n| self.parse_reference(n))?;
        Ok(Quantifier {
            var,
            collection,
            span,
        })
    }

    /// Parse a task.
    fn parse_task(&self, node: Node) -> ParseResult<Task> {
        let span = Span::from_node(&node);

        // Get task ID (first identifier)
        let id = self
            .find_child(&node, "identifier")
            .map(|n| TaskId {
                value: self.text_string(&n),
                span: Span::from_node(&n),
            })?;

        let mut title = Prose::new("", Span::dummy());
        let mut req_refs = Vec::new();
        let mut fields = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        let mut first_prose = true;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "prose" => {
                    if first_prose {
                        title = self.parse_prose(child);
                        first_prose = false;
                    } else {
                        prose.push(self.parse_prose(child));
                    }
                }
                "task_ref_list" => {
                    req_refs = self.parse_task_ref_list(child);
                }
                "task_field" => {
                    if let Ok(field) = self.parse_task_field(child) {
                        fields.push(field);
                    }
                }
                _ => {}
            }
        }

        Ok(Task {
            id,
            title,
            req_refs,
            fields,
            prose,
            span,
        })
    }

    /// Parse task reference list.
    fn parse_task_ref_list(&self, node: Node) -> Vec<ReqId> {
        let mut refs = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                refs.push(ReqId {
                    value: self.text_string(&child),
                    span: Span::from_node(&child),
                });
            }
        }
        refs
    }

    /// Parse a task field.
    fn parse_task_field(&self, node: Node) -> ParseResult<TaskField> {
        let span = Span::from_node(&node);
        let text = self.text(&node);

        let kind = if text.starts_with("file:") {
            TaskFieldKind::File
        } else if text.starts_with("tests:") {
            TaskFieldKind::Tests
        } else if text.starts_with("depends:") {
            TaskFieldKind::Depends
        } else if text.starts_with("status:") {
            TaskFieldKind::Status
        } else if text.starts_with("evidence:") {
            TaskFieldKind::Evidence
        } else {
            TaskFieldKind::Context
        };

        let value = self
            .find_child_opt(&node, "prose")
            .map(|n| self.parse_prose(n))
            .unwrap_or_else(|| Prose::new("", Span::dummy()));

        Ok(TaskField { kind, value, span })
    }

    /// Parse an aesthetic block.
    fn parse_aesthetic(&self, node: Node) -> ParseResult<Aesthetic> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let mut fields = Vec::new();
        let mut prose = Vec::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "aesthetic_field" => {
                    if let Ok(field) = self.parse_aesthetic_field(child) {
                        fields.push(field);
                    }
                }
                "prose" => {
                    prose.push(self.parse_prose(child));
                }
                _ => {}
            }
        }

        Ok(Aesthetic {
            name,
            fields,
            prose,
            span,
        })
    }

    /// Parse an aesthetic field.
    fn parse_aesthetic_field(&self, node: Node) -> ParseResult<AestheticField> {
        let span = Span::from_node(&node);
        let name = self.find_child(&node, "identifier").map(|n| self.parse_identifier(n))?;

        let text = self.text(&node);
        let is_soft = text.contains("[~]");

        let value = self
            .find_child_opt(&node, "prose")
            .map(|n| self.parse_prose(n))
            .unwrap_or_else(|| Prose::new("", Span::dummy()));

        Ok(AestheticField {
            name,
            is_soft,
            value,
            span,
        })
    }

    /// Parse a foreign block.
    fn parse_foreign_block(&self, node: Node) -> ParseResult<ForeignBlock> {
        let span = Span::from_node(&node);

        let language = self
            .find_child_opt(&node, "language")
            .map(|n| self.text_string(&n))
            .unwrap_or_default();

        let mut content = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "prose" {
                content.push(self.parse_prose(child));
            }
        }

        Ok(ForeignBlock {
            language,
            content,
            span,
        })
    }

    // ========================================================================
    // Primitive parsing
    // ========================================================================

    /// Parse an identifier.
    fn parse_identifier(&self, node: Node) -> Identifier {
        Identifier {
            value: self.text_string(&node),
            span: Span::from_node(&node),
        }
    }

    /// Parse a reference.
    fn parse_reference(&self, node: Node) -> Reference {
        // Get the inner identifier (without backticks)
        let name = self
            .find_child_opt(&node, "identifier")
            .map(|n| self.text_string(&n))
            .unwrap_or_else(|| {
                // Fallback: strip backticks manually
                let text = self.text(&node);
                text.trim_matches('`').to_string()
            });

        Reference {
            name,
            span: Span::from_node(&node),
        }
    }

    /// Parse a string literal.
    fn parse_string_lit(&self, node: Node) -> StringLit {
        let text = self.text(&node);
        // Remove surrounding quotes
        let value = text.trim_matches('"');
        StringLit {
            value: value.to_string(),
            span: Span::from_node(&node),
        }
    }

    /// Parse prose.
    fn parse_prose(&self, node: Node) -> Prose {
        Prose {
            text: self.text_string(&node),
            span: Span::from_node(&node),
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    /// Find a required child by kind.
    fn find_child<'b>(&self, node: &'b Node, kind: &'static str) -> ParseResult<Node<'b>> {
        self.find_child_opt(node, kind)
            .ok_or(ParseError::MissingChild { name: kind })
    }

    /// Find an optional child by kind.
    fn find_child_opt<'b>(&self, node: &'b Node, kind: &str) -> Option<Node<'b>> {
        let mut cursor = node.walk();
        node.children(&mut cursor).find(|c| c.kind() == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let result = Parser::parse("");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.spec.is_none());
        assert!(file.sections.is_empty());
    }

    #[test]
    fn parse_spec_decl() {
        let result = Parser::parse("spec TaskManagement\n");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.spec.is_some());
        assert_eq!(file.spec.unwrap().name.value, "TaskManagement");
    }

    #[test]
    fn parse_section() {
        let result = Parser::parse("# Requirements\n\nSome prose here.\n");
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.sections.len(), 1);
        assert_eq!(file.sections[0].header.text, "Requirements");
    }

    #[test]
    fn parse_requirement() {
        let source = r#"# Requirements

## REQ-1: Task Creation
Description here
when: user submits form
the system shall: create task
"#;
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert_eq!(file.sections.len(), 1);

        let section = &file.sections[0];
        let req = section.contents.iter().find_map(|c| {
            if let SectionContent::Requirement(r) = c {
                Some(r)
            } else {
                None
            }
        });
        assert!(req.is_some());
        let req = req.unwrap();
        assert_eq!(req.id.value, "REQ-1");
        assert_eq!(req.ears_clauses.len(), 1);
    }

    #[test]
    fn parse_concept() {
        let source = r#"# Concepts

Concept Task:
  field id (`Identifier`): unique
  field title (`String`)
"#;
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let file = result.unwrap();

        let concept = file.sections[0].contents.iter().find_map(|c| {
            if let SectionContent::Concept(c) = c {
                Some(c)
            } else {
                None
            }
        });
        assert!(concept.is_some());
        let concept = concept.unwrap();
        assert_eq!(concept.name.value, "Task");
        assert_eq!(concept.fields.len(), 2);
    }

    #[test]
    fn parse_task() {
        let source = r#"# Tasks

## TASK-1: Implement Feature [REQ-1]
file: src/main.rs
status: pending
"#;
        let result = Parser::parse(source);
        assert!(result.is_ok());
        let file = result.unwrap();

        let task = file.sections[0].contents.iter().find_map(|c| {
            if let SectionContent::Task(t) = c {
                Some(t)
            } else {
                None
            }
        });
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.id.value, "TASK-1");
        assert_eq!(task.req_refs.len(), 1);
        assert_eq!(task.req_refs[0].value, "REQ-1");
        assert_eq!(task.fields.len(), 2);
    }
}
