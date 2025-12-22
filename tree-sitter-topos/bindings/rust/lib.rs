//! This crate provides topos language support for the [tree-sitter] parsing library.
//!
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter::Language;

unsafe extern "C" {
    fn tree_sitter_topos() -> *const ();
}

/// Get the tree-sitter [Language] for this grammar.
pub fn language() -> Language {
    unsafe { Language::from_raw(tree_sitter_topos() as _) }
}

/// The content of the [`node-types.json`] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers#static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// The symbol highlighting queries.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::language())
            .expect("Error loading topos grammar");
    }
}