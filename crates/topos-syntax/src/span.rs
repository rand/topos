//! Source location tracking for AST nodes.

use facet::Facet;

/// A span representing a range in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Facet)]
pub struct Span {
    /// Start byte offset (0-indexed).
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
    /// Start line (0-indexed).
    pub start_line: u32,
    /// Start column (0-indexed, in bytes).
    pub start_col: u32,
    /// End line (0-indexed).
    pub end_line: u32,
    /// End column (0-indexed, in bytes).
    pub end_col: u32,
}

impl Span {
    /// Create a new span from byte offsets and positions.
    #[must_use]
    pub const fn new(
        start: u32,
        end: u32,
        start_line: u32,
        start_col: u32,
        end_line: u32,
        end_col: u32,
    ) -> Self {
        Self {
            start,
            end,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Create a span from a tree-sitter node.
    #[must_use]
    pub fn from_node(node: &tree_sitter::Node) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            start: node.start_byte() as u32,
            end: node.end_byte() as u32,
            start_line: start.row as u32,
            start_col: start.column as u32,
            end_line: end.row as u32,
            end_col: end.column as u32,
        }
    }

    /// Create a dummy span (for synthesized nodes).
    #[must_use]
    pub const fn dummy() -> Self {
        Self {
            start: 0,
            end: 0,
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        }
    }

    /// Check if this span is a dummy span.
    #[must_use]
    pub const fn is_dummy(&self) -> bool {
        self.start == 0 && self.end == 0
    }

    /// Get the length in bytes.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }

    /// Check if the span is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans to create a span covering both.
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let (start, start_line, start_col) = if self.start <= other.start {
            (self.start, self.start_line, self.start_col)
        } else {
            (other.start, other.start_line, other.start_col)
        };

        let (end, end_line, end_col) = if self.end >= other.end {
            (self.end, self.end_line, self.end_col)
        } else {
            (other.end, other.end_line, other.end_col)
        };

        Self {
            start,
            end,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start_line + 1,
            self.start_col + 1,
            self.end_line + 1,
            self.end_col + 1
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_display() {
        let span = Span::new(0, 10, 0, 0, 0, 10);
        assert_eq!(span.to_string(), "1:1-1:11");
    }

    #[test]
    fn span_merge() {
        let a = Span::new(0, 5, 0, 0, 0, 5);
        let b = Span::new(10, 15, 1, 0, 1, 5);
        let merged = a.merge(&b);
        assert_eq!(merged.start, 0);
        assert_eq!(merged.end, 15);
    }
}
