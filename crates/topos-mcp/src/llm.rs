//! LLM provider abstraction for typed hole suggestions.
//!
//! This module provides a trait-based abstraction for LLM providers,
//! with an implementation for the Anthropic API (Claude).

use serde::{Deserialize, Serialize};

/// Result type for LLM operations.
pub type LlmResult<T> = Result<T, LlmError>;

/// Errors that can occur during LLM operations.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API key not configured: set ANTHROPIC_API_KEY")]
    ApiKeyMissing,

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Rate limited: retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    #[error("Provider unavailable: {0}")]
    Unavailable(String),
}

/// Context gathered for a typed hole suggestion.
#[derive(Debug, Clone, Serialize)]
pub struct HoleContext {
    /// The hole's type hint if present (e.g., "`Input` -> `Output`").
    pub type_hint: Option<String>,

    /// Named hole identifier if present.
    pub name: Option<String>,

    /// Parent context (concept field, behavior signature, etc.).
    pub parent_context: String,

    /// Surrounding code snippet.
    pub surrounding_code: String,

    /// Related concepts referenced nearby.
    pub related_concepts: Vec<String>,

    /// Adjacent constraints that provide additional context.
    pub adjacent_constraints: Vec<String>,

    /// The spec name.
    pub spec_name: Option<String>,
}

/// A suggestion for filling a typed hole.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoleSuggestion {
    /// The suggested replacement text.
    pub replacement: String,

    /// Explanation of why this suggestion fits.
    pub explanation: String,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// Whether this is based on type constraints.
    pub type_based: bool,
}

/// Response from LLM for hole suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionResponse {
    /// List of suggestions, ordered by confidence.
    pub suggestions: Vec<HoleSuggestion>,

    /// Raw model response for debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
}

/// Trait for LLM providers that can generate hole suggestions.
///
/// Note: This trait uses `async fn in trait` which is available in Rust 1.75+.
/// For dyn compatibility, use the concrete `AnthropicProvider` type directly.
pub trait LlmProvider: Send + Sync {
    /// Check if the provider is available (API key set, etc.).
    fn is_available(&self) -> bool;

    /// Get the provider name for logging/debugging.
    fn name(&self) -> &'static str;
}

/// Anthropic API provider using Claude.
pub struct AnthropicProvider {
    api_key: Option<String>,
    model: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from environment.
    ///
    /// Reads `ANTHROPIC_API_KEY` from environment.
    pub fn from_env() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
        Self {
            api_key,
            model: "claude-sonnet-4-20250514".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create with explicit API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
            model: "claude-sonnet-4-20250514".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Set the model to use.
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Build the prompt for hole suggestions.
    fn build_prompt(&self, context: &HoleContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("You are helping complete a typed hole [?] in a Topos specification.\n\n");

        if let Some(ref spec_name) = context.spec_name {
            prompt.push_str(&format!("Spec: {}\n\n", spec_name));
        }

        prompt.push_str("## Hole Context\n\n");
        prompt.push_str(&format!("Location: {}\n", context.parent_context));

        if let Some(ref type_hint) = context.type_hint {
            prompt.push_str(&format!("Type constraint: `{}`\n", type_hint));
        }

        if let Some(ref name) = context.name {
            prompt.push_str(&format!("Hole name: {}\n", name));
        }

        prompt.push_str("\n## Surrounding Code\n\n```topos\n");
        prompt.push_str(&context.surrounding_code);
        prompt.push_str("\n```\n\n");

        if !context.related_concepts.is_empty() {
            prompt.push_str("## Related Concepts\n\n");
            for concept in &context.related_concepts {
                prompt.push_str(&format!("- `{}`\n", concept));
            }
            prompt.push('\n');
        }

        if !context.adjacent_constraints.is_empty() {
            prompt.push_str("## Adjacent Constraints\n\n");
            for constraint in &context.adjacent_constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
            prompt.push('\n');
        }

        prompt.push_str(r#"## Instructions

Suggest 1-3 completions for the typed hole. For each suggestion, provide:
1. The exact replacement text (what should replace the [?])
2. A brief explanation of why it fits
3. Your confidence (0.0-1.0)

Respond in this JSON format:

```json
{
  "suggestions": [
    {
      "replacement": "the replacement text",
      "explanation": "brief explanation",
      "confidence": 0.85,
      "type_based": true
    }
  ]
}
```

Guidelines:
- If there's a type constraint, honor it exactly
- Consider the parent context (what element contains this hole)
- Use terminology consistent with nearby concepts
- Higher confidence for type-constrained holes
- Lower confidence for prose/description holes
"#);

        prompt
    }

    /// Parse the LLM response.
    fn parse_response(&self, response: &str) -> LlmResult<SuggestionResponse> {
        // Extract JSON from response (may be wrapped in code blocks)
        let json_str = extract_json(response);

        let parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| LlmError::InvalidResponse(format!("JSON parse error: {}", e)))?;

        let suggestions = parsed
            .get("suggestions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        Some(HoleSuggestion {
                            replacement: s.get("replacement")?.as_str()?.to_string(),
                            explanation: s
                                .get("explanation")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            confidence: s
                                .get("confidence")
                                .and_then(|v| v.as_f64())
                                .map(|v| v as f32)
                                .unwrap_or(0.5),
                            type_based: s
                                .get("type_based")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(SuggestionResponse {
            suggestions,
            raw_response: Some(response.to_string()),
        })
    }
}

impl LlmProvider for AnthropicProvider {
    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }
}

impl AnthropicProvider {
    /// Send an arbitrary prompt to the Anthropic API and get a text response.
    ///
    /// This is a general-purpose completion method used by semantic analysis.
    pub async fn complete(&self, prompt: &str) -> LlmResult<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(LlmError::ApiKeyMissing)?;

        let request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": 2048,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if response.status() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(LlmError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Extract text content from Claude response
        let text = response_body
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| LlmError::InvalidResponse("No text in response".to_string()))?;

        Ok(text.to_string())
    }

    /// Generate suggestions for a typed hole using the Anthropic API.
    pub async fn suggest_hole(&self, context: &HoleContext) -> LlmResult<SuggestionResponse> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or(LlmError::ApiKeyMissing)?;

        let prompt = self.build_prompt(context);

        let request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": 1024,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        if response.status() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(LlmError::RateLimited {
                retry_after_secs: retry_after,
            });
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        // Extract text content from Claude response
        let text = response_body
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|block| block.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| LlmError::InvalidResponse("No text in response".to_string()))?;

        self.parse_response(text)
    }
}

/// Extract JSON from a response that might have markdown code blocks.
fn extract_json(response: &str) -> String {
    // Try to find JSON in code blocks first
    if let Some(start) = response.find("```json") {
        let json_start = start + 7;
        if let Some(end) = response[json_start..].find("```") {
            return response[json_start..json_start + end].trim().to_string();
        }
    }

    // Try plain code blocks
    if let Some(start) = response.find("```") {
        let after_start = start + 3;
        if let Some(end) = response[after_start..].find("```") {
            let content = &response[after_start..after_start + end];
            let trimmed = content.trim();
            if trimmed.starts_with('{') {
                return trimmed.to_string();
            }
            // Skip language identifier if present
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

/// Get the default LLM provider (Anthropic from environment).
pub fn default_provider() -> AnthropicProvider {
    AnthropicProvider::from_env()
}

/// Fallback suggestions when LLM is unavailable.
pub fn fallback_suggestions(context: &HoleContext) -> SuggestionResponse {
    let mut suggestions = Vec::new();

    // Type-based suggestion
    if let Some(ref type_hint) = context.type_hint {
        suggestions.push(HoleSuggestion {
            replacement: type_hint.clone(),
            explanation: "Use the specified type constraint directly".to_string(),
            confidence: 0.7,
            type_based: true,
        });
    }

    // Context-based suggestions based on parent
    if context.parent_context.contains("field") {
        if context.parent_context.contains("id") {
            suggestions.push(HoleSuggestion {
                replacement: "`UUID`".to_string(),
                explanation: "Common type for ID fields".to_string(),
                confidence: 0.5,
                type_based: false,
            });
        }
        if context.parent_context.contains("date") || context.parent_context.contains("time") {
            suggestions.push(HoleSuggestion {
                replacement: "`DateTime`".to_string(),
                explanation: "Common type for temporal fields".to_string(),
                confidence: 0.5,
                type_based: false,
            });
        }
    }

    if context.parent_context.contains("behavior") && context.parent_context.contains("output") {
        suggestions.push(HoleSuggestion {
            replacement: "`Result<T, E>`".to_string(),
            explanation: "Common pattern for fallible operations".to_string(),
            confidence: 0.4,
            type_based: false,
        });
    }

    // If no specific suggestions, provide a generic one
    if suggestions.is_empty() {
        suggestions.push(HoleSuggestion {
            replacement: "TODO".to_string(),
            explanation: "Placeholder - specify the concrete type or value".to_string(),
            confidence: 0.2,
            type_based: false,
        });
    }

    SuggestionResponse {
        suggestions,
        raw_response: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_code_block() {
        let response = r#"Here's the JSON:

```json
{"suggestions": []}
```

That's it."#;

        let json = extract_json(response);
        assert!(json.starts_with('{'));
        assert!(json.contains("suggestions"));
    }

    #[test]
    fn test_extract_json_raw() {
        let response = r#"{"suggestions": [{"replacement": "test"}]}"#;
        let json = extract_json(response);
        assert_eq!(json, response);
    }

    #[test]
    fn test_fallback_suggestions_with_type() {
        let context = HoleContext {
            type_hint: Some("`String`".to_string()),
            name: None,
            parent_context: "field name in Concept User".to_string(),
            surrounding_code: "field name [?]".to_string(),
            related_concepts: vec![],
            adjacent_constraints: vec![],
            spec_name: None,
        };

        let response = fallback_suggestions(&context);
        assert!(!response.suggestions.is_empty());
        assert!(response.suggestions[0].type_based);
        assert!(response.suggestions[0].replacement.contains("String"));
    }

    #[test]
    fn test_fallback_suggestions_id_field() {
        let context = HoleContext {
            type_hint: None,
            name: None,
            parent_context: "field user_id in Concept Order".to_string(),
            surrounding_code: "field user_id [?]".to_string(),
            related_concepts: vec!["User".to_string()],
            adjacent_constraints: vec![],
            spec_name: None,
        };

        let response = fallback_suggestions(&context);
        assert!(response
            .suggestions
            .iter()
            .any(|s| s.replacement.contains("UUID")));
    }

    #[test]
    fn test_provider_availability() {
        // Without API key, provider should not be available
        // SAFETY: This is test code running in isolation
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };
        let provider = AnthropicProvider::from_env();
        assert!(!provider.is_available());
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_build_prompt() {
        let provider = AnthropicProvider::from_env();
        let context = HoleContext {
            type_hint: Some("`User` -> `Session`".to_string()),
            name: Some("login_result".to_string()),
            parent_context: "output of Behavior login".to_string(),
            surrounding_code: "Behavior login:\n  output: [?login_result : `User` -> `Session`]"
                .to_string(),
            related_concepts: vec!["User".to_string(), "Session".to_string()],
            adjacent_constraints: vec!["user must be authenticated".to_string()],
            spec_name: Some("AuthSpec".to_string()),
        };

        let prompt = provider.build_prompt(&context);
        assert!(prompt.contains("AuthSpec"));
        assert!(prompt.contains("`User` -> `Session`"));
        assert!(prompt.contains("login_result"));
        assert!(prompt.contains("Related Concepts"));
    }
}
