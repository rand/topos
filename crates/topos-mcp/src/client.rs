//! MCP client for LLM-powered semantic analysis.
//!
//! This module provides a client for connecting to MCP servers that expose
//! sampling/completion capabilities, used for semantic drift detection.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Result type for MCP client operations.
pub type ClientResult<T> = Result<T, ClientError>;

/// Errors that can occur during MCP client operations.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Sampling not supported by server")]
    SamplingNotSupported,

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Offline mode: no MCP server available")]
    Offline,
}

/// Configuration for the MCP client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server URL for SSE transport.
    pub server_url: Option<String>,

    /// Command to spawn for stdio transport.
    pub server_command: Option<Vec<String>>,

    /// Maximum tokens for completion responses.
    pub max_tokens: u32,

    /// Temperature for sampling (0.0 = deterministic).
    pub temperature: f32,

    /// Request timeout in seconds.
    pub timeout_secs: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: None,
            server_command: None,
            max_tokens: 2048,
            temperature: 0.0,
            timeout_secs: 60,
        }
    }
}

/// Semantic analysis result from LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticAnalysis {
    /// Alignment score between 0.0 and 1.0.
    pub alignment_score: f32,

    /// List of detected discrepancies.
    pub discrepancies: Vec<Discrepancy>,

    /// Confidence in the analysis (0.0 to 1.0).
    pub confidence: f32,

    /// Whether the result is conclusive.
    pub is_conclusive: bool,

    /// Raw LLM response for debugging.
    pub raw_response: String,
}

/// A discrepancy detected between old and new spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discrepancy {
    /// Type of discrepancy.
    pub kind: DiscrepancyKind,

    /// Identifier of the affected element.
    pub element_id: String,

    /// Description of the discrepancy.
    pub description: String,

    /// Severity level.
    pub severity: DiscrepancySeverity,
}

/// Types of semantic discrepancies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscrepancyKind {
    /// Meaning has changed significantly.
    MeaningChanged,

    /// Constraint weakened (less restrictive).
    ConstraintWeakened,

    /// Constraint strengthened (more restrictive).
    ConstraintStrengthened,

    /// Intent appears different.
    IntentDrift,

    /// Ambiguity introduced.
    AmbiguityIntroduced,

    /// Terminology inconsistency.
    TerminologyChanged,
}

impl DiscrepancyKind {
    /// Get the string representation of this discrepancy kind.
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MeaningChanged => "meaning_changed",
            Self::ConstraintWeakened => "constraint_weakened",
            Self::ConstraintStrengthened => "constraint_strengthened",
            Self::IntentDrift => "intent_drift",
            Self::AmbiguityIntroduced => "ambiguity_introduced",
            Self::TerminologyChanged => "terminology_changed",
        }
    }
}

/// Severity of a discrepancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscrepancySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Cache entry for semantic analysis results.
#[derive(Debug, Clone)]
struct CacheEntry {
    result: SemanticAnalysis,
    #[allow(dead_code)]
    timestamp: std::time::Instant,
}

/// MCP client for semantic analysis.
pub struct McpClient {
    config: ClientConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    connected: Arc<RwLock<bool>>,
}

impl McpClient {
    /// Create a new MCP client with the given configuration.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            connected: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a client from environment variables.
    ///
    /// Checks for:
    /// - `TOPOS_MCP_URL` - Server URL for SSE transport
    /// - `TOPOS_MCP_COMMAND` - Space-separated command for stdio transport
    /// - `TOPOS_OFFLINE` - If set, operates in offline mode
    pub fn from_env() -> Self {
        let server_url = std::env::var("TOPOS_MCP_URL").ok();
        let server_command = std::env::var("TOPOS_MCP_COMMAND")
            .ok()
            .map(|cmd| cmd.split_whitespace().map(String::from).collect());

        let config = ClientConfig {
            server_url,
            server_command,
            ..Default::default()
        };

        Self::new(config)
    }

    /// Check if the client is in offline mode.
    pub fn is_offline(&self) -> bool {
        std::env::var("TOPOS_OFFLINE").is_ok()
            || (self.config.server_url.is_none() && self.config.server_command.is_none())
    }

    /// Connect to the MCP server.
    pub async fn connect(&self) -> ClientResult<()> {
        if self.is_offline() {
            return Err(ClientError::Offline);
        }

        // For now, we just mark as connected
        // Full implementation would establish the connection
        let mut connected = self.connected.write().await;
        *connected = true;

        Ok(())
    }

    /// Perform semantic analysis comparing old and new spec content.
    pub async fn analyze_semantic_drift(
        &self,
        old_content: &str,
        new_content: &str,
        element_type: &str,
        element_id: &str,
    ) -> ClientResult<SemanticAnalysis> {
        if self.is_offline() {
            return Err(ClientError::Offline);
        }

        // Check cache first
        let cache_key = compute_cache_key(old_content, new_content, element_id);
        if let Some(cached) = self.get_cached(&cache_key).await {
            return Ok(cached);
        }

        // Build the prompt
        let prompt = build_semantic_comparison_prompt(old_content, new_content, element_type, element_id);

        // Call the LLM
        let response = self.sample(&prompt).await?;

        // Parse the response
        let analysis = parse_semantic_response(&response)?;

        // Cache the result
        self.cache_result(&cache_key, analysis.clone()).await;

        Ok(analysis)
    }

    /// Send a sampling request to the MCP server.
    async fn sample(&self, _prompt: &str) -> ClientResult<String> {
        // In a full implementation, this would:
        // 1. Connect to the MCP server if not connected
        // 2. Send a CreateMessageRequest with the prompt
        // 3. Wait for the response
        // 4. Extract the text content

        // For now, we simulate the response structure
        // This will be replaced with actual MCP client calls

        if self.is_offline() {
            return Err(ClientError::Offline);
        }

        // Placeholder: In production, this connects to actual MCP server
        // Return an error indicating we need a real server
        Err(ClientError::ConnectionFailed(
            "MCP server connection not yet implemented - use TOPOS_OFFLINE=1 for structural-only mode".to_string()
        ))
    }

    /// Get a cached result if available.
    async fn get_cached(&self, key: &str) -> Option<SemanticAnalysis> {
        let cache = self.cache.read().await;
        cache.get(key).map(|entry| entry.result.clone())
    }

    /// Cache a result.
    async fn cache_result(&self, key: &str, result: SemanticAnalysis) {
        let mut cache = self.cache.write().await;
        cache.insert(
            key.to_string(),
            CacheEntry {
                result,
                timestamp: std::time::Instant::now(),
            },
        );
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

/// Compute a cache key from content hashes.
fn compute_cache_key(old: &str, new: &str, element_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    old.hash(&mut hasher);
    new.hash(&mut hasher);
    element_id.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Build a prompt for semantic comparison.
fn build_semantic_comparison_prompt(
    old_content: &str,
    new_content: &str,
    element_type: &str,
    element_id: &str,
) -> String {
    format!(
        r#"You are analyzing changes to a software specification for semantic drift.

## Task
Compare the OLD and NEW versions of a {element_type} specification element (ID: {element_id}).
Determine if the meaning, intent, or constraints have changed semantically (not just textually).

## OLD Version
```
{old_content}
```

## NEW Version
```
{new_content}
```

## Instructions
Analyze the semantic differences and respond in the following JSON format:

```json
{{
  "alignment_score": <float 0.0-1.0, where 1.0 means semantically identical>,
  "discrepancies": [
    {{
      "kind": "<meaning_changed|constraint_weakened|constraint_strengthened|intent_drift|ambiguity_introduced|terminology_changed>",
      "element_id": "<affected element>",
      "description": "<brief explanation>",
      "severity": "<low|medium|high|critical>"
    }}
  ],
  "confidence": <float 0.0-1.0>,
  "reasoning": "<brief explanation of your analysis>"
}}
```

Focus on:
1. Changes in meaning or intent (not just wording)
2. Constraints that became more or less restrictive
3. New ambiguities or clarifications
4. Terminology changes that affect understanding

If the changes are purely cosmetic (formatting, synonyms with same meaning), the alignment_score should be close to 1.0.
"#,
        element_type = element_type,
        element_id = element_id,
        old_content = old_content,
        new_content = new_content,
    )
}

/// Parse a semantic analysis response from the LLM.
fn parse_semantic_response(response: &str) -> ClientResult<SemanticAnalysis> {
    // Extract JSON from the response (it might be wrapped in markdown code blocks)
    let json_str = extract_json(response);

    // Parse the JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| ClientError::InvalidResponse(format!("JSON parse error: {}", e)))?;

    // Extract fields
    let alignment_score = parsed
        .get("alignment_score")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(0.5);

    let raw_confidence = parsed
        .get("confidence")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(0.5);

    // Adjust confidence based on hedging language
    let confidence = adjust_confidence_for_hedging(response, raw_confidence);

    let discrepancies = parsed
        .get("discrepancies")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| parse_discrepancy(d))
                .collect()
        })
        .unwrap_or_default();

    let is_conclusive = confidence >= 0.7;

    Ok(SemanticAnalysis {
        alignment_score,
        discrepancies,
        confidence,
        is_conclusive,
        raw_response: response.to_string(),
    })
}

/// Extract JSON from a response that might have markdown code blocks.
fn extract_json(response: &str) -> String {
    // Try to find JSON in code blocks first
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start..].find("```\n").or_else(|| response[start..].rfind("```")) {
            let json_start = start + 7; // len("```json")
            let json_end = start + end;
            if json_start < json_end {
                return response[json_start..json_end].trim().to_string();
            }
        }
    }

    // Try plain code blocks
    if let Some(start) = response.find("```") {
        let after_start = start + 3;
        if let Some(end) = response[after_start..].find("```") {
            // Skip language identifier if present
            let content = &response[after_start..after_start + end];
            let trimmed = content.trim();
            if trimmed.starts_with('{') {
                return trimmed.to_string();
            }
            // Skip first line if it's a language identifier
            if let Some(newline) = trimmed.find('\n') {
                return trimmed[newline..].trim().to_string();
            }
        }
    }

    // Look for raw JSON object
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }

    response.to_string()
}

/// Adjust confidence based on hedging language in the response.
fn adjust_confidence_for_hedging(response: &str, base_confidence: f32) -> f32 {
    let hedging_phrases = [
        "might",
        "could be",
        "possibly",
        "perhaps",
        "uncertain",
        "not sure",
        "hard to tell",
        "difficult to determine",
        "may or may not",
        "unclear",
        "ambiguous",
    ];

    let strong_phrases = [
        "clearly",
        "definitely",
        "certainly",
        "obviously",
        "without doubt",
        "unambiguously",
    ];

    let lower = response.to_lowercase();
    let hedge_count = hedging_phrases.iter().filter(|p| lower.contains(*p)).count();
    let strong_count = strong_phrases.iter().filter(|p| lower.contains(*p)).count();

    let adjustment = (strong_count as f32 * 0.05) - (hedge_count as f32 * 0.1);
    (base_confidence + adjustment).clamp(0.0, 1.0)
}

/// Parse a single discrepancy from JSON.
fn parse_discrepancy(value: &serde_json::Value) -> Option<Discrepancy> {
    let kind_str = value.get("kind")?.as_str()?;
    let kind = match kind_str {
        "meaning_changed" => DiscrepancyKind::MeaningChanged,
        "constraint_weakened" => DiscrepancyKind::ConstraintWeakened,
        "constraint_strengthened" => DiscrepancyKind::ConstraintStrengthened,
        "intent_drift" => DiscrepancyKind::IntentDrift,
        "ambiguity_introduced" => DiscrepancyKind::AmbiguityIntroduced,
        "terminology_changed" => DiscrepancyKind::TerminologyChanged,
        _ => return None,
    };

    let element_id = value
        .get("element_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let description = value
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let severity_str = value.get("severity").and_then(|v| v.as_str()).unwrap_or("medium");
    let severity = match severity_str {
        "low" => DiscrepancySeverity::Low,
        "medium" => DiscrepancySeverity::Medium,
        "high" => DiscrepancySeverity::High,
        "critical" => DiscrepancySeverity::Critical,
        _ => DiscrepancySeverity::Medium,
    };

    Some(Discrepancy {
        kind,
        element_id,
        description,
        severity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_deterministic() {
        let key1 = compute_cache_key("old", "new", "REQ-1");
        let key2 = compute_cache_key("old", "new", "REQ-1");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_different_content() {
        let key1 = compute_cache_key("old1", "new", "REQ-1");
        let key2 = compute_cache_key("old2", "new", "REQ-1");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let response = r#"Here's the analysis:

```json
{"alignment_score": 0.85}
```

That's my assessment."#;

        let json = extract_json(response);
        assert!(json.contains("alignment_score"));
        assert!(json.starts_with('{'));
    }

    #[test]
    fn test_extract_json_raw() {
        let response = r#"{"alignment_score": 0.9, "discrepancies": []}"#;
        let json = extract_json(response);
        assert_eq!(json, response);
    }

    #[test]
    fn test_parse_semantic_response() {
        let response = r#"```json
{
    "alignment_score": 0.75,
    "discrepancies": [
        {
            "kind": "meaning_changed",
            "element_id": "REQ-1",
            "description": "The timeout changed from 30s to 60s",
            "severity": "medium"
        }
    ],
    "confidence": 0.9,
    "reasoning": "Clear numeric change in constraint"
}
```"#;

        let analysis = parse_semantic_response(response).unwrap();
        assert!((analysis.alignment_score - 0.75).abs() < 0.01);
        assert_eq!(analysis.discrepancies.len(), 1);
        assert_eq!(analysis.discrepancies[0].kind, DiscrepancyKind::MeaningChanged);
        assert!(analysis.is_conclusive);
    }

    #[test]
    fn test_hedging_reduces_confidence() {
        let base = 0.8;
        let hedged = "I'm uncertain about this, it might be different, possibly changed";
        let confident = "This is clearly different, definitely a change";

        let hedged_conf = adjust_confidence_for_hedging(hedged, base);
        let confident_conf = adjust_confidence_for_hedging(confident, base);

        assert!(hedged_conf < base);
        assert!(confident_conf >= base);
    }

    #[test]
    fn test_build_prompt_includes_content() {
        let prompt = build_semantic_comparison_prompt(
            "old content here",
            "new content here",
            "requirement",
            "REQ-1",
        );

        assert!(prompt.contains("old content here"));
        assert!(prompt.contains("new content here"));
        assert!(prompt.contains("REQ-1"));
        assert!(prompt.contains("requirement"));
    }

    #[test]
    fn test_client_offline_mode() {
        let config = ClientConfig::default();
        let client = McpClient::new(config);
        assert!(client.is_offline());
    }

    #[test]
    fn test_discrepancy_kind_variants() {
        assert_eq!(DiscrepancyKind::MeaningChanged.as_str(), "meaning_changed");
        assert_eq!(DiscrepancyKind::ConstraintWeakened.as_str(), "constraint_weakened");
    }
}
