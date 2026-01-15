//! Salsa database definition for incremental analysis.

use std::sync::Arc;

use salsa::Setter;

use crate::anchors::{extract_anchors, AnchorCollection};

/// The Salsa database trait for Topos analysis.
#[salsa::db]
pub trait Db: salsa::Database {}

/// Input storage for Topos spec source files.
#[salsa::input]
pub struct SourceFile {
    /// The file path (used as key).
    #[returns(ref)]
    pub path: String,

    /// The source text content.
    #[returns(ref)]
    pub text: String,
}

/// Input storage for Rust source files (for anchor extraction).
#[salsa::input]
pub struct RustSourceFile {
    /// The file path (used as key).
    #[returns(ref)]
    pub path: String,

    /// The source text content.
    #[returns(ref)]
    pub text: String,
}

/// The concrete Salsa database implementation.
#[salsa::db]
#[derive(Default, Clone)]
pub struct AnalysisDatabase {
    storage: salsa::Storage<Self>,
}

impl AnalysisDatabase {
    /// Create a new database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a Topos spec source file to the database.
    pub fn add_file(&mut self, path: String, text: String) -> SourceFile {
        SourceFile::new(self, path, text)
    }

    /// Update a Topos spec source file's text.
    pub fn update_file(&mut self, file: SourceFile, text: String) {
        file.set_text(self).to(text);
    }

    /// Add a Rust source file to the database (for anchor extraction).
    pub fn add_rust_file(&mut self, path: String, text: String) -> RustSourceFile {
        RustSourceFile::new(self, path, text)
    }

    /// Update a Rust source file's text.
    pub fn update_rust_file(&mut self, file: RustSourceFile, text: String) {
        file.set_text(self).to(text);
    }
}

#[salsa::db]
impl salsa::Database for AnalysisDatabase {}

#[salsa::db]
impl Db for AnalysisDatabase {}

/// Parse a Topos spec source file into a typed AST.
#[salsa::tracked]
pub fn parse(db: &dyn Db, file: SourceFile) -> Arc<topos_syntax::SourceFile> {
    let text = file.text(db);
    match topos_syntax::Parser::parse(text) {
        Ok(ast) => Arc::new(ast),
        Err(_) => {
            // Return empty AST on parse failure
            Arc::new(topos_syntax::SourceFile {
                spec: None,
                imports: vec![],
                sections: vec![],
                prose: vec![],
                span: topos_syntax::Span::dummy(),
            })
        }
    }
}

/// Parse anchors from a Rust source file.
///
/// This is a Salsa tracked function that enables incremental re-parsing
/// when Rust source files change.
#[salsa::tracked]
pub fn parse_anchors(db: &dyn Db, file: RustSourceFile) -> Arc<AnchorCollection> {
    let text = file.text(db);
    let path = file.path(db);
    Arc::new(extract_anchors(text, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let _db = AnalysisDatabase::new();
    }

    #[test]
    fn test_add_file() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_file("test.tps".to_string(), "spec Test\n".to_string());
        assert_eq!(file.path(&db), "test.tps");
        assert_eq!(file.text(&db), "spec Test\n");
    }

    #[test]
    fn test_parse_file() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_file("test.tps".to_string(), "spec Example\n".to_string());
        let ast = parse(&db, file);
        assert!(ast.spec.is_some());
    }

    #[test]
    fn test_add_rust_file() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_rust_file(
            "test.rs".to_string(),
            "// @topos(concept=\"User\")\npub struct User {}\n".to_string(),
        );
        assert_eq!(file.path(&db), "test.rs");
    }

    #[test]
    fn test_parse_anchors() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_rust_file(
            "test.rs".to_string(),
            r#"
// @topos(concept="User")
pub struct User {
    // @topos(field="id")
    pub id: u64,
}
"#
            .to_string(),
        );
        let anchors = parse_anchors(&db, file);
        assert_eq!(anchors.len(), 2);
        assert!(anchors.concept("User").is_some());
    }

    #[test]
    fn test_incremental_anchor_parsing() {
        let mut db = AnalysisDatabase::new();
        let file = db.add_rust_file(
            "test.rs".to_string(),
            "// @topos(concept=\"Order\")\npub struct Order {}\n".to_string(),
        );

        // First parse
        let anchors1 = parse_anchors(&db, file);
        assert_eq!(anchors1.len(), 1);

        // Update the file
        db.update_rust_file(
            file,
            r#"
// @topos(concept="Order")
pub struct Order {}

// @topos(concept="Item")
pub struct Item {}
"#
            .to_string(),
        );

        // Second parse should reflect the change
        let anchors2 = parse_anchors(&db, file);
        assert_eq!(anchors2.len(), 2);
    }
}
