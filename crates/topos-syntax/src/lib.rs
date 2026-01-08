//! Topos syntax parsing and typed AST.
//!
//! This crate provides:
//! - A typed AST for Topos specifications
//! - A parser that converts tree-sitter CST to the typed AST
//! - Source span tracking for error reporting and IDE features
//!
//! # Example
//!
//! ```
//! use topos_syntax::Parser;
//!
//! let source = "spec Example\n";
//!
//! let result = Parser::parse(source);
//! assert!(result.is_ok());
//! let file = result.unwrap();
//! assert!(file.spec.is_some());
//! ```

pub mod ast;
pub mod format;
pub mod parser;
pub mod span;

#[cfg(test)]
mod proptest_support;

pub use ast::*;
pub use format::{format, FormatConfig};
pub use parser::{ParseError, ParseResult, Parser};
pub use span::Span;
