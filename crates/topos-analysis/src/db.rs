//! Salsa database definition for incremental analysis.

use std::sync::Arc;

use salsa::Setter;

/// The Salsa database trait for Topos analysis.
#[salsa::db]
pub trait Db: salsa::Database {}

/// Input storage for source files.
#[salsa::input]
pub struct SourceFile {
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

    /// Add a source file to the database.
    pub fn add_file(&mut self, path: String, text: String) -> SourceFile {
        SourceFile::new(self, path, text)
    }

    /// Update a source file's text.
    pub fn update_file(&mut self, file: SourceFile, text: String) {
        file.set_text(self).to(text);
    }
}

#[salsa::db]
impl salsa::Database for AnalysisDatabase {}

#[salsa::db]
impl Db for AnalysisDatabase {}

/// Parse a source file into a typed AST.
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
}
