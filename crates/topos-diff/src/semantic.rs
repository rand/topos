//! Semantic drift detection using LLM-as-Judge.
//!
//! This module extends structural diffing with semantic analysis
//! to detect meaning changes in prose content.

use serde::Serialize;

use crate::strategy::{ComparisonStrategy, ElementType};
use crate::DiffReport;

/// Result of semantic drift analysis.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticDiffReport {
    /// The underlying structural diff.
    pub structural: DiffReport,

    /// Semantic analysis results for modified elements.
    pub semantic_results: Vec<SemanticElementResult>,

    /// Overall alignment score (0.0 to 1.0).
    pub overall_alignment: f32,

    /// Overall confidence in the analysis.
    pub overall_confidence: f32,

    /// Whether the analysis is conclusive.
    pub is_conclusive: bool,

    /// Strategy used for comparison.
    pub strategy: String,

    /// Whether semantic analysis was available.
    pub semantic_available: bool,
}

impl SemanticDiffReport {
    /// Create a structural-only report (when MCP is unavailable).
    pub fn structural_only(structural: DiffReport) -> Self {
        Self {
            structural,
            semantic_results: Vec::new(),
            overall_alignment: 1.0, // Assume aligned if we can't check
            overall_confidence: 0.0, // No confidence without semantic analysis
            is_conclusive: false,
            strategy: "structural".to_string(),
            semantic_available: false,
        }
    }

    /// Check if there are any differences.
    pub fn has_changes(&self) -> bool {
        !self.structural.is_empty() || !self.semantic_results.is_empty()
    }

    /// Get elements with semantic drift (alignment < threshold).
    pub fn drifted_elements(&self, threshold: f32) -> Vec<&SemanticElementResult> {
        self.semantic_results
            .iter()
            .filter(|r| r.alignment_score < threshold)
            .collect()
    }

    /// Format as human-readable text.
    pub fn format_text(&self) -> String {
        let mut out = String::new();

        // Header
        out.push_str(&format!(
            "Drift Report (strategy: {}, semantic: {})\n",
            self.strategy,
            if self.semantic_available { "available" } else { "unavailable" }
        ));
        out.push_str(&"=".repeat(50));
        out.push('\n');

        // Structural changes
        if !self.structural.is_empty() {
            out.push_str("\n## Structural Changes\n\n");
            out.push_str(&self.structural.format_text());
        }

        // Semantic analysis
        if !self.semantic_results.is_empty() {
            out.push_str("\n## Semantic Analysis\n\n");

            for result in &self.semantic_results {
                let status = if result.alignment_score >= 0.9 {
                    "✓ aligned"
                } else if result.alignment_score >= 0.7 {
                    "~ minor drift"
                } else {
                    "✗ significant drift"
                };

                out.push_str(&format!(
                    "- **{}** ({}): {:.0}% aligned {}\n",
                    result.element_id,
                    result.element_type,
                    result.alignment_score * 100.0,
                    status
                ));

                for discrepancy in &result.discrepancies {
                    out.push_str(&format!(
                        "    - [{}] {}: {}\n",
                        discrepancy.severity,
                        discrepancy.kind,
                        discrepancy.description
                    ));
                }
            }

            out.push_str(&format!(
                "\nOverall alignment: {:.0}% (confidence: {:.0}%)\n",
                self.overall_alignment * 100.0,
                self.overall_confidence * 100.0
            ));

            if !self.is_conclusive {
                out.push_str("⚠ Low confidence - results may be inconclusive\n");
            }
        }

        if self.structural.is_empty() && self.semantic_results.is_empty() {
            out.push_str("\nNo differences found.\n");
        }

        out
    }

    /// Format as JSON.
    pub fn format_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Semantic analysis result for a single element.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticElementResult {
    /// Element identifier.
    pub element_id: String,

    /// Element type (requirement, behavior, etc.).
    pub element_type: String,

    /// Alignment score (0.0 to 1.0).
    pub alignment_score: f32,

    /// Detected discrepancies.
    pub discrepancies: Vec<SemanticDiscrepancy>,

    /// Confidence in this analysis.
    pub confidence: f32,
}

/// A semantic discrepancy.
#[derive(Debug, Clone, Serialize)]
pub struct SemanticDiscrepancy {
    /// Type of discrepancy.
    pub kind: String,

    /// Description.
    pub description: String,

    /// Severity level.
    pub severity: String,
}

/// Options for semantic diff.
#[derive(Debug, Clone)]
pub struct SemanticDiffOptions {
    /// Comparison strategy to use.
    pub strategy: ComparisonStrategy,

    /// Alignment threshold for flagging drift.
    pub alignment_threshold: f32,

    /// Whether to fall back to structural on MCP failure.
    pub fallback_on_error: bool,
}

impl Default for SemanticDiffOptions {
    fn default() -> Self {
        Self {
            strategy: ComparisonStrategy::Hybrid,
            alignment_threshold: 0.7,
            fallback_on_error: true,
        }
    }
}

/// Perform semantic diff between two spec sources.
///
/// This is an async function that may call MCP for semantic analysis.
/// If MCP is unavailable and `fallback_on_error` is true, returns
/// structural-only results.
pub async fn semantic_diff(
    old_source: &str,
    new_source: &str,
    options: SemanticDiffOptions,
) -> Result<SemanticDiffReport, String> {
    // First, get structural diff
    let structural = crate::diff_specs(old_source, new_source)?;

    // If strategy is structural-only, return early
    if options.strategy == ComparisonStrategy::Structural {
        return Ok(SemanticDiffReport::structural_only(structural));
    }

    // Try to perform semantic analysis
    match perform_semantic_analysis(&structural, old_source, new_source, &options).await {
        Ok(report) => Ok(report),
        Err(e) if options.fallback_on_error => {
            tracing::warn!("Semantic analysis failed, falling back to structural: {}", e);
            Ok(SemanticDiffReport::structural_only(structural))
        }
        Err(e) => Err(e),
    }
}

/// Synchronous version that only does structural comparison.
pub fn semantic_diff_sync(
    old_source: &str,
    new_source: &str,
    _options: SemanticDiffOptions,
) -> Result<SemanticDiffReport, String> {
    let structural = crate::diff_specs(old_source, new_source)?;
    Ok(SemanticDiffReport::structural_only(structural))
}

/// Perform semantic analysis on structural differences.
async fn perform_semantic_analysis(
    structural: &DiffReport,
    old_source: &str,
    new_source: &str,
    options: &SemanticDiffOptions,
) -> Result<SemanticDiffReport, String> {
    // Check if MCP client is available
    let client = topos_mcp::client::McpClient::from_env();

    if client.is_offline() {
        return Err("MCP client is offline".to_string());
    }

    // Connect to MCP server
    client.connect().await.map_err(|e| e.to_string())?;

    let mut semantic_results = Vec::new();

    // Analyze modified requirements
    for req_diff in &structural.modified_requirements {
        if should_analyze_semantically(ElementType::Requirement, &options.strategy) {
            // Extract requirement content from both sources
            let old_content = extract_requirement_content(old_source, &req_diff.id);
            let new_content = extract_requirement_content(new_source, &req_diff.id);

            if let (Some(old), Some(new)) = (old_content, new_content) {
                match client
                    .analyze_semantic_drift(&old, &new, "requirement", &req_diff.id)
                    .await
                {
                    Ok(analysis) => {
                        semantic_results.push(SemanticElementResult {
                            element_id: req_diff.id.clone(),
                            element_type: "requirement".to_string(),
                            alignment_score: analysis.alignment_score,
                            discrepancies: analysis
                                .discrepancies
                                .into_iter()
                                .map(|d| SemanticDiscrepancy {
                                    kind: format!("{:?}", d.kind),
                                    description: d.description,
                                    severity: format!("{:?}", d.severity).to_lowercase(),
                                })
                                .collect(),
                            confidence: analysis.confidence,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Semantic analysis failed for {}: {}",
                            req_diff.id,
                            e
                        );
                        // Use structural info as fallback
                        semantic_results.push(SemanticElementResult {
                            element_id: req_diff.id.clone(),
                            element_type: "requirement".to_string(),
                            alignment_score: 0.5, // Unknown - structural changes detected
                            discrepancies: req_diff
                                .changes
                                .iter()
                                .map(|c| SemanticDiscrepancy {
                                    kind: "structural".to_string(),
                                    description: c.clone(),
                                    severity: "info".to_string(),
                                })
                                .collect(),
                            confidence: 0.0,
                        });
                    }
                }
            }
        }
    }

    // Analyze modified behaviors
    for beh_diff in &structural.modified_behaviors {
        if should_analyze_semantically(ElementType::Behavior, &options.strategy) {
            let old_content = extract_behavior_content(old_source, &beh_diff.name);
            let new_content = extract_behavior_content(new_source, &beh_diff.name);

            if let (Some(old), Some(new)) = (old_content, new_content) {
                match client
                    .analyze_semantic_drift(&old, &new, "behavior", &beh_diff.name)
                    .await
                {
                    Ok(analysis) => {
                        semantic_results.push(SemanticElementResult {
                            element_id: beh_diff.name.clone(),
                            element_type: "behavior".to_string(),
                            alignment_score: analysis.alignment_score,
                            discrepancies: analysis
                                .discrepancies
                                .into_iter()
                                .map(|d| SemanticDiscrepancy {
                                    kind: format!("{:?}", d.kind),
                                    description: d.description,
                                    severity: format!("{:?}", d.severity).to_lowercase(),
                                })
                                .collect(),
                            confidence: analysis.confidence,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Semantic analysis failed for {}: {}",
                            beh_diff.name,
                            e
                        );
                        semantic_results.push(SemanticElementResult {
                            element_id: beh_diff.name.clone(),
                            element_type: "behavior".to_string(),
                            alignment_score: 0.5,
                            discrepancies: beh_diff
                                .changes
                                .iter()
                                .map(|c| SemanticDiscrepancy {
                                    kind: "structural".to_string(),
                                    description: c.clone(),
                                    severity: "info".to_string(),
                                })
                                .collect(),
                            confidence: 0.0,
                        });
                    }
                }
            }
        }
    }

    // Calculate overall metrics
    let (overall_alignment, overall_confidence) = if semantic_results.is_empty() {
        (1.0, 1.0)
    } else {
        let total_alignment: f32 = semantic_results.iter().map(|r| r.alignment_score).sum();
        let total_confidence: f32 = semantic_results.iter().map(|r| r.confidence).sum();
        let count = semantic_results.len() as f32;
        (total_alignment / count, total_confidence / count)
    };

    let is_conclusive = overall_confidence >= 0.7;

    Ok(SemanticDiffReport {
        structural: structural.clone(),
        semantic_results,
        overall_alignment,
        overall_confidence,
        is_conclusive,
        strategy: options.strategy.name().to_string(),
        semantic_available: true,
    })
}

/// Extract the content of a requirement from source text.
fn extract_requirement_content(source: &str, req_id: &str) -> Option<String> {
    // Look for requirement header pattern: ## REQ-N: Title
    let pattern = format!("## {}:", req_id);
    let start_idx = source.find(&pattern)?;

    // Find the end of this requirement (next ## or # or end of file)
    let content_start = start_idx;
    let rest = &source[start_idx..];

    // Find next section header
    let end_idx = rest[3..] // Skip the initial "## "
        .find("\n## ")
        .or_else(|| rest[3..].find("\n# "))
        .map(|i| start_idx + 3 + i)
        .unwrap_or(source.len());

    Some(source[content_start..end_idx].trim().to_string())
}

/// Extract the content of a behavior from source text.
fn extract_behavior_content(source: &str, behavior_name: &str) -> Option<String> {
    // Look for behavior pattern: Behavior Name:
    let pattern = format!("Behavior {}:", behavior_name);
    let start_idx = source.find(&pattern)?;

    // Find the end of this behavior (next Behavior, Concept, #, or double newline)
    let content_start = start_idx;
    let rest = &source[start_idx..];

    let end_idx = rest[1..]
        .find("\nBehavior ")
        .or_else(|| rest[1..].find("\nConcept "))
        .or_else(|| rest[1..].find("\n# "))
        .or_else(|| rest[1..].find("\n\n\n"))
        .map(|i| start_idx + 1 + i)
        .unwrap_or(source.len());

    Some(source[content_start..end_idx].trim().to_string())
}

/// Check if an element type should be analyzed semantically given the strategy.
fn should_analyze_semantically(element_type: ElementType, strategy: &ComparisonStrategy) -> bool {
    match strategy {
        ComparisonStrategy::Structural => false,
        ComparisonStrategy::Semantic => true,
        ComparisonStrategy::Hybrid => {
            // In hybrid mode, analyze prose-heavy elements
            matches!(
                element_type,
                ElementType::Requirement | ElementType::Behavior | ElementType::Invariant
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structural_only_report() {
        let structural = DiffReport::default();
        let report = SemanticDiffReport::structural_only(structural);

        assert!(!report.semantic_available);
        assert_eq!(report.strategy, "structural");
        assert!(!report.is_conclusive);
    }

    #[test]
    fn test_has_changes_empty() {
        let report = SemanticDiffReport::structural_only(DiffReport::default());
        assert!(!report.has_changes());
    }

    #[test]
    fn test_has_changes_structural() {
        let mut structural = DiffReport::default();
        structural.added_requirements.push("REQ-1".to_string());
        let report = SemanticDiffReport::structural_only(structural);
        assert!(report.has_changes());
    }

    #[test]
    fn test_drifted_elements() {
        let mut report = SemanticDiffReport::structural_only(DiffReport::default());
        report.semantic_results.push(SemanticElementResult {
            element_id: "REQ-1".to_string(),
            element_type: "requirement".to_string(),
            alignment_score: 0.5,
            discrepancies: Vec::new(),
            confidence: 0.8,
        });
        report.semantic_results.push(SemanticElementResult {
            element_id: "REQ-2".to_string(),
            element_type: "requirement".to_string(),
            alignment_score: 0.9,
            discrepancies: Vec::new(),
            confidence: 0.8,
        });

        let drifted = report.drifted_elements(0.7);
        assert_eq!(drifted.len(), 1);
        assert_eq!(drifted[0].element_id, "REQ-1");
    }

    #[test]
    fn test_should_analyze_semantically() {
        assert!(!should_analyze_semantically(
            ElementType::Concept,
            &ComparisonStrategy::Hybrid
        ));
        assert!(should_analyze_semantically(
            ElementType::Requirement,
            &ComparisonStrategy::Hybrid
        ));
        assert!(should_analyze_semantically(
            ElementType::Behavior,
            &ComparisonStrategy::Semantic
        ));
        assert!(!should_analyze_semantically(
            ElementType::Behavior,
            &ComparisonStrategy::Structural
        ));
    }

    #[test]
    fn test_format_text() {
        let report = SemanticDiffReport::structural_only(DiffReport::default());
        let text = report.format_text();
        assert!(text.contains("structural"));
        assert!(text.contains("unavailable"));
    }

    #[test]
    fn test_sync_diff() {
        let old = "spec Old\n";
        let new = "spec New\n";
        let options = SemanticDiffOptions::default();

        let report = semantic_diff_sync(old, new, options).unwrap();
        assert!(!report.semantic_available);
    }

    #[test]
    fn test_extract_requirement_content() {
        let source = r#"spec Test

# Requirements

## REQ-1: First requirement
This is the first requirement.
It has multiple lines.

## REQ-2: Second requirement
This is another requirement.
"#;

        let content = extract_requirement_content(source, "REQ-1");
        assert!(content.is_some());
        let content = content.unwrap();
        assert!(content.contains("REQ-1: First requirement"));
        assert!(content.contains("multiple lines"));
        assert!(!content.contains("REQ-2"));

        let content2 = extract_requirement_content(source, "REQ-2");
        assert!(content2.is_some());
        assert!(content2.unwrap().contains("another requirement"));

        // Non-existent requirement
        assert!(extract_requirement_content(source, "REQ-99").is_none());
    }

    #[test]
    fn test_extract_behavior_content() {
        let source = r#"spec Test

# Behaviors

Behavior create_order:
  when: user submits order
  then: order is created

Behavior cancel_order:
  when: user cancels
  then: order is cancelled
"#;

        let content = extract_behavior_content(source, "create_order");
        assert!(content.is_some());
        let content = content.unwrap();
        assert!(content.contains("create_order"));
        assert!(content.contains("user submits order"));
        assert!(!content.contains("cancel_order"));

        let content2 = extract_behavior_content(source, "cancel_order");
        assert!(content2.is_some());
        assert!(content2.unwrap().contains("user cancels"));

        // Non-existent behavior
        assert!(extract_behavior_content(source, "unknown").is_none());
    }
}
