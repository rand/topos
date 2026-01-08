//! Formatter for Topos specifications.
//!
//! This module provides formatting/pretty-printing for Topos source files.
//! It maintains semantic equivalence while applying consistent styling.

use crate::ast::*;

/// Configuration for the formatter.
#[derive(Debug, Clone)]
pub struct FormatConfig {
    /// Indentation string (default: two spaces).
    pub indent: String,
    /// Maximum line width before wrapping (default: 100).
    pub max_width: usize,
    /// Add blank lines between major sections (default: true).
    pub blank_between_sections: bool,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            max_width: 100,
            blank_between_sections: true,
        }
    }
}

/// Format a source file to a string.
pub fn format(file: &SourceFile, config: &FormatConfig) -> String {
    let mut formatter = Formatter::new(config.clone());
    formatter.format_file(file);
    formatter.output
}

/// The internal formatter state.
struct Formatter {
    config: FormatConfig,
    output: String,
    at_line_start: bool,
}

impl Formatter {
    fn new(config: FormatConfig) -> Self {
        Self {
            config,
            output: String::new(),
            at_line_start: true,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        self.at_line_start = s.ends_with('\n');
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
        self.at_line_start = true;
    }

    fn blank_line(&mut self) {
        if !self.output.ends_with("\n\n") && !self.output.is_empty() {
            self.output.push('\n');
        }
    }

    fn format_file(&mut self, file: &SourceFile) {
        // Spec declaration
        if let Some(spec) = &file.spec {
            self.writeln(&format!("spec {}", spec.name.value));
            self.blank_line();
        }

        // Imports
        for import in &file.imports {
            self.format_import(import);
        }
        if !file.imports.is_empty() {
            self.blank_line();
        }

        // Sections
        for (i, section) in file.sections.iter().enumerate() {
            if i > 0 && self.config.blank_between_sections {
                self.blank_line();
            }
            self.format_section(section);
        }
    }

    fn format_import(&mut self, import: &Import) {
        match &import.kind {
            ImportKind::Items { from, items } => {
                if let Some(path) = from {
                    self.write(&format!("import from \"{}\": ", path.value));
                } else {
                    self.write("import: ");
                }
                let item_strs: Vec<_> = items
                    .iter()
                    .map(|item| {
                        if let Some(alias) = &item.alias {
                            format!("`{}` as {}", item.reference.name, alias.value)
                        } else {
                            format!("`{}`", item.reference.name)
                        }
                    })
                    .collect();
                self.writeln(&item_strs.join(", "));
            }
            ImportKind::Module { path, alias } => {
                self.writeln(&format!("import \"{}\" as {}", path.value, alias.value));
            }
        }
    }

    fn format_section(&mut self, section: &Section) {
        // Section header
        self.writeln(&format!("# {}", section.header.text.trim()));
        self.blank_line();

        // Section contents
        for content in &section.contents {
            self.format_section_content(content);
        }
    }

    fn format_section_content(&mut self, content: &SectionContent) {
        match content {
            SectionContent::Requirement(req) => self.format_requirement(req),
            SectionContent::Concept(concept) => self.format_concept(concept),
            SectionContent::Behavior(behavior) => self.format_behavior(behavior),
            SectionContent::Invariant(invariant) => self.format_invariant(invariant),
            SectionContent::Task(task) => self.format_task(task),
            SectionContent::Aesthetic(aesthetic) => self.format_aesthetic(aesthetic),
            SectionContent::ForeignBlock(block) => self.format_foreign_block(block),
            SectionContent::Subsection(sub) => self.format_subsection(sub),
            SectionContent::Prose(prose) => {
                if !prose.text.trim().is_empty() {
                    self.writeln(prose.text.trim());
                }
            }
        }
    }

    fn format_requirement(&mut self, req: &Requirement) {
        // Header
        self.writeln(&format!("## {}: {}", req.id.value, req.title.text.trim()));

        // EARS clauses
        for ears in &req.ears_clauses {
            self.writeln(&format!("{}when: {}", self.config.indent, ears.when.text.trim()));
            self.writeln(&format!(
                "{}the system shall: {}",
                self.config.indent,
                ears.shall.text.trim()
            ));
        }

        // Acceptance criteria
        if let Some(acceptance) = &req.acceptance {
            self.writeln(&format!("{}acceptance:", self.config.indent));
            for clause in &acceptance.clauses {
                let kind = match clause.kind {
                    AcceptanceKind::Given => "given",
                    AcceptanceKind::When => "when",
                    AcceptanceKind::Then => "then",
                };
                self.writeln(&format!(
                    "{}{}{}: {}",
                    self.config.indent,
                    self.config.indent,
                    kind,
                    clause.content.text.trim()
                ));
            }
        }

        // Additional prose
        for prose in &req.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }

    fn format_concept(&mut self, concept: &Concept) {
        self.writeln(&format!("Concept {}:", concept.name.value));

        for field in &concept.fields {
            self.format_field(field);
        }

        for prose in &concept.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }

    fn format_field(&mut self, field: &Field) {
        let mut line = format!("{}field {}", self.config.indent, field.name.value);

        if let Some(ty) = &field.ty {
            line.push_str(&format!(" ({})", self.format_type_expr(ty)));
        }

        self.writeln(&line);

        // Constraints
        for constraint in &field.constraints {
            self.format_constraint(constraint);
        }
    }

    fn format_type_expr(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Reference(r) => format!("`{}`", r.name),
            TypeExpr::Hole(h) => {
                if let Some(content) = &h.content {
                    format!("[? {}]", content)
                } else {
                    "[?]".to_string()
                }
            }
            TypeExpr::List { element, .. } => format!("List of `{}`", element.name),
            TypeExpr::Optional { inner, .. } => format!("Optional `{}`", inner.name),
            TypeExpr::Applied { base, arg, .. } => format!("`{}` `{}`", base.name, arg.name),
            TypeExpr::OneOf { variants, .. } => {
                let names: Vec<_> = variants.iter().map(|v| v.value.as_str()).collect();
                format!("one of: {}", names.join(", "))
            }
        }
    }

    fn format_constraint(&mut self, constraint: &Constraint) {
        let line = match constraint {
            Constraint::Unique { .. } => format!("{}{}unique", self.config.indent, self.config.indent),
            Constraint::Default { value, .. } => {
                format!("{}{}default: {}", self.config.indent, self.config.indent, value.text.trim())
            }
            Constraint::Derived { expr, .. } => {
                format!("{}{}derived: {}", self.config.indent, self.config.indent, expr.text.trim())
            }
            Constraint::Invariant { predicate, .. } => {
                format!(
                    "{}{}invariant: {}",
                    self.config.indent,
                    self.config.indent,
                    predicate.text.trim()
                )
            }
            Constraint::AtLeast { count, unit, .. } => {
                if let Some(u) = unit {
                    format!("{}{}at least {} {}", self.config.indent, self.config.indent, count, u.value)
                } else {
                    format!("{}{}at least {}", self.config.indent, self.config.indent, count)
                }
            }
        };
        self.writeln(&line);
    }

    fn format_behavior(&mut self, behavior: &Behavior) {
        let implements: Vec<_> = behavior.implements.iter().map(|r| r.value.as_str()).collect();
        if implements.is_empty() {
            self.writeln(&format!("Behavior {}:", behavior.name.value));
        } else {
            self.writeln(&format!(
                "Behavior {} [{}]:",
                behavior.name.value,
                implements.join(", ")
            ));
        }

        for given in &behavior.given {
            self.writeln(&format!("{}given: {}", self.config.indent, given.text.trim()));
        }

        if let Some(returns) = &behavior.returns {
            self.writeln(&format!("{}returns: {}", self.config.indent, returns.text.trim()));
        }

        for req in &behavior.requires {
            self.writeln(&format!("{}requires: {}", self.config.indent, req.text.trim()));
        }

        for ens in &behavior.ensures {
            self.writeln(&format!("{}ensures: {}", self.config.indent, ens.text.trim()));
        }

        for ears in &behavior.ears_clauses {
            self.writeln(&format!("{}when: {}", self.config.indent, ears.when.text.trim()));
            self.writeln(&format!(
                "{}the system shall: {}",
                self.config.indent,
                ears.shall.text.trim()
            ));
        }

        for prose in &behavior.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }

    fn format_invariant(&mut self, invariant: &Invariant) {
        self.writeln(&format!("Invariant {}:", invariant.name.value));

        for quant in &invariant.quantifiers {
            self.writeln(&format!(
                "{}for each {} in `{}`:",
                self.config.indent,
                quant.var.value,
                quant.collection.name
            ));
        }

        for prose in &invariant.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(&format!("{}{}", self.config.indent, prose.text.trim()));
            }
        }

        self.blank_line();
    }

    fn format_task(&mut self, task: &Task) {
        let req_refs: Vec<_> = task.req_refs.iter().map(|r| r.value.as_str()).collect();
        if req_refs.is_empty() {
            self.writeln(&format!("## {}: {}", task.id.value, task.title.text.trim()));
        } else {
            self.writeln(&format!(
                "## {}: {} [{}]",
                task.id.value,
                task.title.text.trim(),
                req_refs.join(", ")
            ));
        }

        for field in &task.fields {
            let kind = match field.kind {
                TaskFieldKind::File => "file",
                TaskFieldKind::Tests => "tests",
                TaskFieldKind::Depends => "depends",
                TaskFieldKind::Status => "status",
                TaskFieldKind::Evidence => "evidence",
                TaskFieldKind::Context => "context",
            };
            self.writeln(&format!("{}: {}", kind, field.value.text.trim()));
        }

        for prose in &task.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }

    fn format_aesthetic(&mut self, aesthetic: &Aesthetic) {
        self.writeln(&format!("Aesthetic {}:", aesthetic.name.value));

        for field in &aesthetic.fields {
            let soft = if field.is_soft { "[~] " } else { "" };
            self.writeln(&format!(
                "{}{}: {}{}",
                self.config.indent,
                field.name.value,
                soft,
                field.value.text.trim()
            ));
        }

        for prose in &aesthetic.prose {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }

    fn format_foreign_block(&mut self, block: &ForeignBlock) {
        self.writeln(&format!("```{}", block.language));
        for prose in &block.content {
            self.writeln(&prose.text);
        }
        self.writeln("```");
        self.blank_line();
    }

    fn format_subsection(&mut self, sub: &Subsection) {
        self.writeln(&format!("## {}", sub.header.text.trim()));

        for prose in &sub.body {
            if !prose.text.trim().is_empty() {
                self.writeln(prose.text.trim());
            }
        }

        self.blank_line();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;

    #[test]
    fn test_format_empty_spec() {
        let source = "spec Test\n";
        let file = Parser::parse(source).unwrap();
        let formatted = format(&file, &FormatConfig::default());
        assert!(formatted.starts_with("spec Test"));
    }

    #[test]
    fn test_format_preserves_spec_name() {
        let source = "spec   MyApp  \n";
        let file = Parser::parse(source).unwrap();
        let formatted = format(&file, &FormatConfig::default());
        assert!(formatted.contains("spec MyApp"));
    }

    #[test]
    fn test_format_requirement() {
        let source = r#"spec Test

# Requirements

## REQ-1: User Login
Users must be able to log in.
"#;
        let file = Parser::parse(source).unwrap();
        let formatted = format(&file, &FormatConfig::default());
        assert!(formatted.contains("## REQ-1:"));
        assert!(formatted.contains("User Login"));
    }

    #[test]
    fn test_format_concept() {
        let source = r#"spec Test

# Concepts

Concept User:
  field name (`String`)
  field email (`String`)
"#;
        let file = Parser::parse(source).unwrap();
        let formatted = format(&file, &FormatConfig::default());
        assert!(formatted.contains("Concept User:"));
        assert!(formatted.contains("field name"));
    }
}
