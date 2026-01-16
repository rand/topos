//! Test the suggest_hole functionality with LLM
use topos_mcp::llm::{default_provider, fallback_suggestions, HoleContext, LlmProvider};

#[tokio::main]
async fn main() {
    let context = HoleContext {
        type_hint: None,
        name: None,
        parent_context: "field expires_at in Concept Session".to_string(),
        surrounding_code: r#"Concept Session:
  field id (`UUID`)
  field user_id (`UUID`)
  field created_at (`Timestamp`)
  field expires_at ([?])"#.to_string(),
        related_concepts: vec!["Timestamp".to_string(), "Session".to_string()],
        adjacent_constraints: vec![],
        spec_name: Some("AuthSystem".to_string()),
    };

    let provider = default_provider();
    
    if provider.is_available() {
        println!("Using LLM for suggestions...\n");
        match provider.suggest_hole(&context).await {
            Ok(response) => {
                println!("=== LLM Suggestions ===\n");
                for (i, suggestion) in response.suggestions.iter().enumerate() {
                    println!("{}. {} (confidence: {:.0}%)", 
                        i + 1, 
                        suggestion.replacement,
                        suggestion.confidence * 100.0
                    );
                    println!("   {}", suggestion.explanation);
                    println!("   Type-based: {}\n", suggestion.type_based);
                }
            }
            Err(e) => {
                println!("LLM error: {}", e);
                println!("\nFalling back to heuristics...");
                let response = fallback_suggestions(&context);
                for suggestion in &response.suggestions {
                    println!("- {} (confidence: {:.0}%)", suggestion.replacement, suggestion.confidence * 100.0);
                }
            }
        }
    } else {
        println!("No API key - using fallback suggestions:\n");
        let response = fallback_suggestions(&context);
        for suggestion in &response.suggestions {
            println!("- {} (confidence: {:.0}%)", suggestion.replacement, suggestion.confidence * 100.0);
            println!("  {}\n", suggestion.explanation);
        }
    }
}
