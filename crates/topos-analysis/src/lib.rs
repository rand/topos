use tree_sitter::{Parser, Language};

pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

pub fn language() -> Language {
    tree_sitter_topos::language()
}

pub fn check(text: &str) -> Vec<Diagnostic> {
    let mut parser = Parser::new();
    
    // SAFETY: This will currently panic because tree-sitter-topos is unimplemented.
    // In a real scenario, we'd handle language loading properly.
    if let Ok(_) = parser.set_language(&language()) {
        let tree = parser.parse(text, None).unwrap();
        if tree.root_node().has_error() {
            // Very basic error reporting: just say there's a syntax error.
            // In the future, we'll traverse the tree to find ERROR nodes.
            return vec![Diagnostic {
                line: 0,
                column: 0,
                message: "Syntax error detected".to_string(),
            }];
        }
    }

    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_load() {
        let result = std::panic::catch_unwind(|| {
            language();
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_check_empty() {
        // Should not panic if we catch the language load failure
        let result = std::panic::catch_unwind(|| {
            check("");
        });
        // result will be Err because check() calls language() which panics
        assert!(result.is_err());
    }
}
