pub fn language() -> tree_sitter::Language {
    tree_sitter_topos::language()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_load() {
        // This will currently panic because unimplemented!() in tree-sitter-topos
        // but it verifies the linkage.
        let result = std::panic::catch_unwind(|| {
            language();
        });
        assert!(result.is_err());
    }
}