//! Comparison strategy selection for spec drift detection.
//!
//! This module implements strategy selection for comparing spec elements:
//! - **Structural**: Pure AST-based comparison (V1 behavior)
//! - **Semantic**: LLM-based meaning comparison
//! - **Hybrid**: Structural + semantic for prose content

use serde::{Deserialize, Serialize};

/// Comparison strategy for drift detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ComparisonStrategy {
    /// Pure structural comparison (AST-based).
    /// Fast, deterministic, no external dependencies.
    Structural,

    /// Pure semantic comparison (LLM-based).
    /// Understands meaning, but slower and requires MCP server.
    Semantic,

    /// Hybrid: structural for structure, semantic for prose.
    /// Best of both worlds, default choice.
    #[default]
    Hybrid,
}

impl ComparisonStrategy {
    /// Get the strategy for a specific element type.
    ///
    /// Strategy selection rules:
    /// - Concept → Structural (fields are structural)
    /// - Behavior (no prose) → Structural
    /// - Behavior (with prose) → Hybrid
    /// - Requirement → Hybrid (titles/descriptions are prose)
    /// - Task → Structural (mostly metadata)
    /// - Invariant → Semantic (pure constraints in prose)
    pub fn for_element(element_type: ElementType, has_prose: bool) -> Self {
        match (element_type, has_prose) {
            (ElementType::Concept, _) => Self::Structural,
            (ElementType::Task, _) => Self::Structural,
            (ElementType::Behavior, false) => Self::Structural,
            (ElementType::Behavior, true) => Self::Hybrid,
            (ElementType::Requirement, _) => Self::Hybrid,
            (ElementType::Invariant, _) => Self::Semantic,
        }
    }

    /// Check if this strategy requires an MCP server.
    pub fn requires_mcp(&self) -> bool {
        matches!(self, Self::Semantic | Self::Hybrid)
    }

    /// Get the fallback strategy when MCP is unavailable.
    pub fn fallback(&self) -> Self {
        match self {
            Self::Structural => Self::Structural,
            Self::Semantic => Self::Structural,
            Self::Hybrid => Self::Structural,
        }
    }

    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Structural => "structural",
            Self::Semantic => "semantic",
            Self::Hybrid => "hybrid",
        }
    }
}

impl std::fmt::Display for ComparisonStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for ComparisonStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "structural" => Ok(Self::Structural),
            "semantic" => Ok(Self::Semantic),
            "hybrid" => Ok(Self::Hybrid),
            _ => Err(format!("Unknown strategy: {}", s)),
        }
    }
}

/// Types of spec elements for strategy selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Requirement,
    Concept,
    Behavior,
    Task,
    Invariant,
}

impl ElementType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Concept => "concept",
            Self::Behavior => "behavior",
            Self::Task => "task",
            Self::Invariant => "invariant",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_for_concept() {
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Concept, false),
            ComparisonStrategy::Structural
        );
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Concept, true),
            ComparisonStrategy::Structural
        );
    }

    #[test]
    fn test_strategy_for_behavior() {
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Behavior, false),
            ComparisonStrategy::Structural
        );
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Behavior, true),
            ComparisonStrategy::Hybrid
        );
    }

    #[test]
    fn test_strategy_for_requirement() {
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Requirement, false),
            ComparisonStrategy::Hybrid
        );
    }

    #[test]
    fn test_strategy_for_invariant() {
        assert_eq!(
            ComparisonStrategy::for_element(ElementType::Invariant, true),
            ComparisonStrategy::Semantic
        );
    }

    #[test]
    fn test_fallback() {
        assert_eq!(ComparisonStrategy::Semantic.fallback(), ComparisonStrategy::Structural);
        assert_eq!(ComparisonStrategy::Hybrid.fallback(), ComparisonStrategy::Structural);
        assert_eq!(ComparisonStrategy::Structural.fallback(), ComparisonStrategy::Structural);
    }

    #[test]
    fn test_requires_mcp() {
        assert!(!ComparisonStrategy::Structural.requires_mcp());
        assert!(ComparisonStrategy::Semantic.requires_mcp());
        assert!(ComparisonStrategy::Hybrid.requires_mcp());
    }

    #[test]
    fn test_parse_strategy() {
        assert_eq!("structural".parse::<ComparisonStrategy>().unwrap(), ComparisonStrategy::Structural);
        assert_eq!("SEMANTIC".parse::<ComparisonStrategy>().unwrap(), ComparisonStrategy::Semantic);
        assert_eq!("Hybrid".parse::<ComparisonStrategy>().unwrap(), ComparisonStrategy::Hybrid);
        assert!("unknown".parse::<ComparisonStrategy>().is_err());
    }

    #[test]
    fn test_default_strategy() {
        assert_eq!(ComparisonStrategy::default(), ComparisonStrategy::Hybrid);
    }
}
