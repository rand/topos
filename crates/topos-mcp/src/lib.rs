//! MCP (Model Context Protocol) server and client for Topos.
//!
//! ## Server
//!
//! Provides AI-accessible tools for working with Topos specifications:
//! - `validate_spec` - Validate a spec file and return diagnostics
//! - `summarize_spec` - Get an AI-friendly summary of a spec
//! - `compile_context` - Compile task-focused context
//! - `suggest_hole` - Get LLM-powered suggestions for typed holes
//! - `extract_spec` - Extract Topos spec from annotated Rust code
//!
//! ## Client
//!
//! Provides MCP client for LLM-powered semantic analysis:
//! - Semantic drift detection between spec versions
//! - LLM-as-Judge for prose requirement verification

pub mod client;
pub mod llm;

use std::sync::Arc;

use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, ListToolsResult, PaginatedRequestParam,
        ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext, ErrorData, RoleServer, ServerHandler, ServiceExt,
};
use serde_json::{json, Map, Value};

use topos_analysis::AnalysisDatabase;
use topos_context::{compile_context, format_context, CompileOptions, OutputFormat};

/// Create a successful CallToolResult with text content.
fn tool_result(text: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(text.into())],
        is_error: Some(false),
        meta: None,
        structured_content: None,
    }
}

/// Create an error CallToolResult with text content.
fn tool_error(text: impl Into<String>) -> CallToolResult {
    CallToolResult {
        content: vec![Content::text(text.into())],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

/// The Topos MCP server.
#[derive(Debug, Clone, Default)]
pub struct ToposServer;

impl ToposServer {
    pub fn new() -> Self {
        Self
    }

    /// Handle validate_spec tool call.
    fn validate_spec(&self, args: &Value) -> CallToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return tool_error("Error: Missing 'path' argument"),
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Error: Failed to read file {}: {}", path, e)),
        };

        let diagnostics = topos_analysis::check(&content);

        let output = if diagnostics.is_empty() {
            format!("No errors found in {}", path)
        } else {
            let mut out = format!("Found {} issue(s) in {}:\n\n", diagnostics.len(), path);
            for diag in &diagnostics {
                let severity = match diag.severity {
                    topos_analysis::Severity::Error => "ERROR",
                    topos_analysis::Severity::Warning => "WARNING",
                    topos_analysis::Severity::Info => "INFO",
                };
                out.push_str(&format!(
                    "- [{}] Line {}: {}\n",
                    severity,
                    diag.line + 1,
                    diag.message
                ));
            }
            out
        };

        tool_result(output)
    }

    /// Handle summarize_spec tool call.
    fn summarize_spec(&self, args: &Value) -> CallToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return tool_error("Error: Missing 'path' argument"),
        };

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Error: Failed to read file {}: {}", path, e)),
        };

        let mut db = AnalysisDatabase::new();
        let file = db.add_file(path.to_string(), content);

        let symbols = topos_analysis::compute_symbols(&db, file);
        let trace = topos_analysis::compute_traceability(&db, file);

        let untasked: Vec<_> = trace.untasked_requirements().collect();

        let mut summary = format!("# Specification Summary: {}\n\n", path);

        // Requirements
        summary.push_str("## Requirements\n\n");
        if symbols.requirements.is_empty() {
            summary.push_str("No requirements defined.\n\n");
        } else {
            for id in symbols.requirements.keys() {
                let tasks: Vec<_> = trace.tasks_for_req(id).collect();
                let status = if tasks.is_empty() { " (no tasks)" } else { "" };
                summary.push_str(&format!("- **{}**{}\n", id, status));
            }
            summary.push('\n');
        }

        // Concepts
        summary.push_str("## Concepts\n\n");
        if symbols.concepts.is_empty() {
            summary.push_str("No concepts defined.\n\n");
        } else {
            for name in symbols.concepts.keys() {
                summary.push_str(&format!("- {}\n", name));
            }
            summary.push('\n');
        }

        // Tasks
        summary.push_str("## Tasks\n\n");
        if symbols.tasks.is_empty() {
            summary.push_str("No tasks defined.\n\n");
        } else {
            for id in symbols.tasks.keys() {
                let reqs: Vec<_> = trace.reqs_for_task(id).collect();
                if reqs.is_empty() {
                    summary.push_str(&format!("- **{}** (no linked requirements)\n", id));
                } else {
                    summary.push_str(&format!("- **{}** → {}\n", id, reqs.join(", ")));
                }
            }
            summary.push('\n');
        }

        // Traceability
        summary.push_str("## Traceability\n\n");
        summary.push_str(&format!(
            "- **Requirements**: {} total\n",
            symbols.requirements.len()
        ));
        summary.push_str(&format!(
            "- **Without tasks**: {} ({})\n",
            untasked.len(),
            if untasked.is_empty() {
                "all covered".to_string()
            } else {
                untasked.join(", ")
            }
        ));
        summary.push_str(&format!("- **Tasks**: {}\n", symbols.tasks.len()));
        summary.push_str(&format!("- **Concepts**: {}\n", symbols.concepts.len()));
        summary.push_str(&format!("- **Behaviors**: {}\n", symbols.behaviors.len()));

        tool_result(summary)
    }

    /// Handle compile_context tool call.
    fn compile_context_tool(&self, args: &Value) -> CallToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return tool_error("Error: Missing 'path' argument"),
        };

        let task_id = match args.get("task_id").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return tool_error("Error: Missing 'task_id' argument"),
        };

        let format_str = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown");

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Error: Failed to read file {}: {}", path, e)),
        };

        let mut db = AnalysisDatabase::new();
        let file = db.add_file(path.to_string(), content);

        let format = match format_str {
            "json" => OutputFormat::Json,
            "cursor" => OutputFormat::Cursor,
            "windsurf" => OutputFormat::Windsurf,
            "cline" => OutputFormat::Cline,
            _ => OutputFormat::Markdown,
        };

        let options = CompileOptions {
            include_behaviors: true,
            include_descriptions: true,
            ..Default::default()
        };

        match compile_context(&db, file, task_id, options) {
            Some(ctx) => tool_result(format_context(&ctx, format)),
            None => tool_error(format!("Error: Task '{}' not found in {}", task_id, path)),
        }
    }

    /// Handle suggest_hole tool call.
    ///
    /// Returns LLM-powered suggestions for filling a typed hole at the specified position.
    /// Falls back to heuristic suggestions if LLM is unavailable.
    async fn suggest_hole(&self, args: &Value) -> CallToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return tool_error("Error: Missing 'path' argument"),
        };

        // Position can be specified by line/column or offset
        let line = args.get("line").and_then(|v| v.as_u64()).map(|v| v as u32);
        let column = args.get("column").and_then(|v| v.as_u64()).map(|v| v as u32);
        let offset = args.get("offset").and_then(|v| v.as_u64()).map(|v| v as u32);

        // Whether to use LLM (default true if available)
        let use_llm = args
            .get("use_llm")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return tool_error(format!("Error: Failed to read file {}: {}", path, e)),
        };

        let mut db = AnalysisDatabase::new();
        let file = db.add_file(path.to_string(), content.clone());

        // Extract all holes from the file
        let holes = topos_analysis::extract_holes(&db, file);

        if holes.is_empty() {
            return tool_error("No typed holes found in the specification");
        }

        // Find the specific hole
        let hole = if let (Some(line), Some(column)) = (line, column) {
            holes.find_at(line, column)
        } else if let Some(off) = offset {
            holes.find_at_offset(off)
        } else {
            // If no position specified, use the first hole
            holes.holes.first()
        };

        let hole = match hole {
            Some(h) => h,
            None => return tool_error("No hole found at the specified position"),
        };

        // Build surrounding code context
        let span = hole.span();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = span.start_line.saturating_sub(5) as usize;
        let end_line = (span.end_line + 5).min(lines.len() as u32) as usize;
        let surrounding_code: String = lines[start_line..end_line].join("\n");

        // Build the parent context string
        let parent_context = match &hole.parent {
            topos_analysis::HoleParent::ConceptField { concept_name, field_name } => {
                format!("field {} in Concept {}", field_name, concept_name)
            }
            topos_analysis::HoleParent::BehaviorSignature { behavior_name, position } => {
                format!("{} of Behavior {}", position.description(), behavior_name)
            }
            topos_analysis::HoleParent::BehaviorReturns { behavior_name } => {
                format!("returns clause of Behavior {}", behavior_name)
            }
            topos_analysis::HoleParent::BehaviorConstraint { behavior_name, constraint_kind } => {
                format!("{} constraint of Behavior {}", constraint_kind, behavior_name)
            }
            topos_analysis::HoleParent::Unknown => "unknown context".to_string(),
        };

        // Get spec name from parsed AST
        let parsed = topos_analysis::db::parse(&db, file);
        let spec_name = parsed.spec.as_ref().map(|s| s.name.value.clone());

        // Build HoleContext for LLM
        let hole_context = llm::HoleContext {
            type_hint: hole.type_hint.clone(),
            name: hole.name.clone(),
            parent_context,
            surrounding_code,
            related_concepts: hole.related_concepts.clone(),
            adjacent_constraints: hole.adjacent_constraints.clone(),
            spec_name,
        };

        // Try LLM suggestions first
        let suggestions = if use_llm {
            let provider = llm::default_provider();
            if llm::LlmProvider::is_available(&provider) {
                match provider.suggest_hole(&hole_context).await {
                    Ok(response) => {
                        tracing::info!("LLM suggestions received: {} suggestions", response.suggestions.len());
                        Some(response)
                    }
                    Err(e) => {
                        tracing::warn!("LLM suggestion failed, using fallback: {}", e);
                        None
                    }
                }
            } else {
                tracing::debug!("LLM provider not available, using fallback");
                None
            }
        } else {
            None
        };

        // Use LLM suggestions or fall back to heuristics
        let suggestions = suggestions.unwrap_or_else(|| llm::fallback_suggestions(&hole_context));

        // Format output
        let mut output = String::new();
        output.push_str("# Typed Hole Suggestions\n\n");
        output.push_str(&format!("## Hole Context\n\n{}\n", hole.prompt_context()));

        // Show surrounding code
        output.push_str("## Surrounding Code\n\n```topos\n");
        let start_line = span.start_line.saturating_sub(3) as usize;
        let end_line = (span.end_line + 3).min(lines.len() as u32) as usize;
        for (i, line) in lines.iter().enumerate().skip(start_line).take(end_line - start_line) {
            let marker = if i as u32 >= span.start_line && i as u32 <= span.end_line {
                ">>>"
            } else {
                "   "
            };
            output.push_str(&format!("{} {:4} | {}\n", marker, i + 1, line));
        }
        output.push_str("```\n\n");

        // Show suggestions
        output.push_str("## Suggestions\n\n");

        if suggestions.suggestions.is_empty() {
            output.push_str("No suggestions available.\n");
        } else {
            for (i, suggestion) in suggestions.suggestions.iter().enumerate() {
                let confidence_bar = "█".repeat((suggestion.confidence * 10.0) as usize);
                let confidence_empty = "░".repeat(10 - (suggestion.confidence * 10.0) as usize);
                output.push_str(&format!(
                    "{}. **`{}`**\n   {}\n   Confidence: {} {:.0}%{}\n\n",
                    i + 1,
                    suggestion.replacement,
                    suggestion.explanation,
                    confidence_bar + &confidence_empty,
                    suggestion.confidence * 100.0,
                    if suggestion.type_based { " (type-based)" } else { "" }
                ));
            }
        }

        // Add source indicator
        if suggestions.raw_response.is_some() {
            output.push_str("\n*Suggestions powered by LLM*\n");
        } else {
            output.push_str("\n*Heuristic suggestions (set ANTHROPIC_API_KEY for LLM-powered suggestions)*\n");
        }

        tool_result(output)
    }

    /// Handle extract_spec tool call.
    ///
    /// Extracts Topos spec from Rust source files using `@topos()` annotations.
    /// Supports filtering, validation against existing spec, and diff mode.
    fn extract_spec(&self, args: &Value) -> CallToolResult {
        let paths = match args.get("paths") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => return tool_error("Error: Missing 'paths' argument (string or array of strings)"),
        };

        if paths.is_empty() {
            return tool_error("Error: No paths provided");
        }

        let spec_name = args
            .get("spec_name")
            .and_then(|v| v.as_str())
            .unwrap_or("ExtractedSpec");

        // Filter options
        let filter_kind = args.get("filter_kind").and_then(|v| v.as_str());
        let filter_concept = args.get("filter_concept").and_then(|v| v.as_str());
        let filter_requirement = args.get("filter_requirement").and_then(|v| v.as_str());
        let filter_behavior = args.get("filter_behavior").and_then(|v| v.as_str());

        // Validation options
        let compare_spec_path = args.get("compare_spec").and_then(|v| v.as_str());
        let validate = args.get("validate").and_then(|v| v.as_bool()).unwrap_or(false);

        // Expand glob patterns and collect all Rust files
        let mut rust_files = Vec::new();
        for path in &paths {
            if path.contains('*') {
                // Glob pattern - expand it
                if let Ok(entries) = glob::glob(path) {
                    for entry in entries.flatten() {
                        if entry.extension().map_or(false, |e| e == "rs") {
                            rust_files.push(entry);
                        }
                    }
                }
            } else {
                let p = std::path::Path::new(path);
                if p.is_file() && p.extension().map_or(false, |e| e == "rs") {
                    rust_files.push(p.to_path_buf());
                } else if p.is_dir() {
                    // Recursively find .rs files
                    if let Ok(entries) = glob::glob(&format!("{}/**/*.rs", path)) {
                        for entry in entries.flatten() {
                            rust_files.push(entry);
                        }
                    }
                }
            }
        }

        if rust_files.is_empty() {
            return tool_error("Error: No Rust files found in the provided paths");
        }

        // Extract anchors from all files
        let mut collection = topos_analysis::extract_anchors_from_files(&rust_files);

        if collection.is_empty() {
            return tool_error("No @topos() annotations found in the provided files");
        }

        // Apply filters
        if filter_kind.is_some() || filter_concept.is_some() || filter_requirement.is_some() || filter_behavior.is_some() {
            collection = self.filter_anchors(collection, filter_kind, filter_concept, filter_requirement, filter_behavior);
            if collection.is_empty() {
                return tool_error("No anchors match the specified filters");
            }
        }

        // Build output with summary
        let mut output = String::new();
        output.push_str(&format!("# Extracted Specification: {}\n\n", spec_name));
        output.push_str(&format!("Extracted from {} file(s):\n", rust_files.len()));
        for f in &rust_files {
            output.push_str(&format!("- {}\n", f.display()));
        }
        output.push_str(&format!("\nFound {} anchor(s):\n", collection.len()));
        output.push_str(&format!(
            "- {} concept(s)\n",
            collection.concepts().count()
        ));
        output.push_str(&format!(
            "- {} behavior(s)\n",
            collection.behaviors().count()
        ));
        output.push_str(&format!(
            "- {} field(s)\n",
            collection
                .anchors
                .iter()
                .filter(|a| a.kind == topos_analysis::AnchorKind::Field)
                .count()
        ));

        // Validation against existing spec
        if validate || compare_spec_path.is_some() {
            if let Some(spec_path) = compare_spec_path {
                match std::fs::read_to_string(spec_path) {
                    Ok(spec_content) => {
                        let mut db = AnalysisDatabase::new();
                        let file = db.add_file(spec_path.to_string(), spec_content);
                        let symbols = topos_analysis::compute_symbols(&db, file);

                        let validation = topos_analysis::validate_anchors(&collection, &symbols);

                        output.push_str("\n## Validation Results\n\n");

                        if !validation.invalid.is_empty() {
                            output.push_str(&format!("### ❌ Invalid Anchors ({} found)\n\n", validation.invalid.len()));
                            output.push_str("These anchors reference spec elements that don't exist:\n\n");
                            for invalid in &validation.invalid {
                                output.push_str(&format!(
                                    "- `{}` at {}:{} - references undefined '{}'\n",
                                    invalid.anchor.kind_str(),
                                    invalid.anchor.file_path,
                                    invalid.anchor.line + 1,
                                    invalid.unresolved_reference
                                ));
                                if !invalid.suggestions.is_empty() {
                                    output.push_str(&format!("  Did you mean: {}?\n", invalid.suggestions.join(", ")));
                                }
                            }
                            output.push('\n');
                        }

                        if !validation.orphan_spec_elements.is_empty() {
                            output.push_str(&format!("### ⚠️ Orphan Spec Elements ({} found)\n\n", validation.orphan_spec_elements.len()));
                            output.push_str("These spec elements have no code anchor:\n\n");
                            for orphan in &validation.orphan_spec_elements {
                                output.push_str(&format!("- {} `{}`\n", orphan.kind_str(), orphan.name));
                            }
                            output.push('\n');
                        }

                        if !validation.valid.is_empty() {
                            output.push_str(&format!("### ✅ Valid Anchors ({} found)\n\n", validation.valid.len()));
                        }

                        // Identify new elements (in code but not in spec)
                        let new_concepts: Vec<_> = collection.concepts()
                            .filter(|a| a.concept_name().map_or(false, |c| !symbols.concepts.contains_key(c)))
                            .collect();
                        let new_behaviors: Vec<_> = collection.behaviors()
                            .filter(|a| a.behavior_name().map_or(false, |b| !symbols.behaviors.contains_key(b)))
                            .collect();

                        if !new_concepts.is_empty() || !new_behaviors.is_empty() {
                            output.push_str("### 🆕 New Elements (not in spec)\n\n");
                            output.push_str("These anchors define elements not yet in the spec:\n\n");
                            for anchor in &new_concepts {
                                if let Some(concept) = anchor.concept_name() {
                                    output.push_str(&format!("- Concept `{}` at {}:{}\n", concept, anchor.file_path, anchor.line + 1));
                                }
                            }
                            for anchor in &new_behaviors {
                                if let Some(behavior) = anchor.behavior_name() {
                                    output.push_str(&format!("- Behavior `{}` at {}:{}\n", behavior, anchor.file_path, anchor.line + 1));
                                }
                            }
                            output.push('\n');
                        }
                    }
                    Err(e) => {
                        output.push_str(&format!("\n⚠️ Could not read comparison spec: {}\n\n", e));
                    }
                }
            }
        }

        output.push_str("\n---\n\n");

        // Generate the spec
        let spec = collection.generate_spec(spec_name);
        output.push_str(&spec);

        tool_result(output)
    }

    /// Filter anchors based on criteria.
    fn filter_anchors(
        &self,
        collection: topos_analysis::AnchorCollection,
        filter_kind: Option<&str>,
        filter_concept: Option<&str>,
        filter_requirement: Option<&str>,
        filter_behavior: Option<&str>,
    ) -> topos_analysis::AnchorCollection {
        let filtered = collection
            .anchors
            .into_iter()
            .filter(|anchor| {
                // Filter by kind
                if let Some(kind) = filter_kind {
                    let matches_kind = match kind.to_lowercase().as_str() {
                        "concept" => anchor.kind == topos_analysis::AnchorKind::Concept,
                        "behavior" => anchor.kind == topos_analysis::AnchorKind::Behavior,
                        "field" => anchor.kind == topos_analysis::AnchorKind::Field,
                        _ => true,
                    };
                    if !matches_kind {
                        return false;
                    }
                }

                // Filter by concept name
                if let Some(concept_filter) = filter_concept {
                    if let Some(concept) = anchor.concept_name() {
                        if !concept.contains(concept_filter) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                // Filter by requirement
                if let Some(req_filter) = filter_requirement {
                    let has_req = anchor.req_id().map_or(false, |r| r.contains(req_filter))
                        || anchor.implements().iter().any(|i| i.contains(req_filter));
                    if !has_req {
                        return false;
                    }
                }

                // Filter by behavior name
                if let Some(behavior_filter) = filter_behavior {
                    if let Some(behavior) = anchor.behavior_name() {
                        if !behavior.contains(behavior_filter) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            });

        topos_analysis::AnchorCollection::from_anchors(filtered)
    }
}

impl ServerHandler for ToposServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(Default::default()),
                ..Default::default()
            },
            server_info: rmcp::model::Implementation {
                name: "topos-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some(
                "Topos MCP server provides tools for validating and analyzing Topos specifications."
                    .into(),
            ),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: vec![
                make_tool(
                    "validate_spec",
                    "Validate a Topos specification file (.tps) and return any syntax or semantic errors",
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the Topos specification file (.tps)"
                            }
                        },
                        "required": ["path"]
                    }),
                ),
                make_tool(
                    "summarize_spec",
                    "Get a structured summary of a Topos specification including requirements, concepts, tasks, and traceability",
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the Topos specification file (.tps)"
                            }
                        },
                        "required": ["path"]
                    }),
                ),
                make_tool(
                    "compile_context",
                    "Compile AI-focused context for a specific task, including linked requirements and related concepts",
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the Topos specification file (.tps)"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Task ID (e.g., TASK-AUTH-1)"
                            },
                            "format": {
                                "type": "string",
                                "description": "Output format: markdown, json, cursor, windsurf, or cline",
                                "default": "markdown"
                            }
                        },
                        "required": ["path", "task_id"]
                    }),
                ),
                make_tool(
                    "suggest_hole",
                    "Get LLM-powered suggestions for filling a typed hole ([?]) in a Topos specification. Uses Claude for intelligent suggestions based on context, with fallback to heuristics.",
                    json!({
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to the Topos specification file (.tps)"
                            },
                            "line": {
                                "type": "integer",
                                "description": "Line number (0-indexed) where the hole is located"
                            },
                            "column": {
                                "type": "integer",
                                "description": "Column number (0-indexed) where the hole is located"
                            },
                            "offset": {
                                "type": "integer",
                                "description": "Byte offset of the hole in the file (alternative to line/column)"
                            },
                            "use_llm": {
                                "type": "boolean",
                                "description": "Whether to use LLM for suggestions (default: true). Set to false for faster heuristic-only suggestions.",
                                "default": true
                            }
                        },
                        "required": ["path"]
                    }),
                ),
                make_tool(
                    "extract_spec",
                    "Extract a Topos specification from Rust source files using @topos() annotations. Supports filtering by kind/concept/requirement/behavior and validation against existing spec.",
                    json!({
                        "type": "object",
                        "properties": {
                            "paths": {
                                "oneOf": [
                                    { "type": "string" },
                                    { "type": "array", "items": { "type": "string" } }
                                ],
                                "description": "Path(s) to Rust files or directories. Supports glob patterns (e.g., 'src/**/*.rs')"
                            },
                            "spec_name": {
                                "type": "string",
                                "description": "Name for the generated specification",
                                "default": "ExtractedSpec"
                            },
                            "filter_kind": {
                                "type": "string",
                                "enum": ["concept", "behavior", "field"],
                                "description": "Filter anchors by kind (concept, behavior, or field)"
                            },
                            "filter_concept": {
                                "type": "string",
                                "description": "Filter to anchors matching this concept name (substring match)"
                            },
                            "filter_requirement": {
                                "type": "string",
                                "description": "Filter to anchors referencing this requirement (e.g., 'REQ-AUTH')"
                            },
                            "filter_behavior": {
                                "type": "string",
                                "description": "Filter to anchors matching this behavior name (substring match)"
                            },
                            "compare_spec": {
                                "type": "string",
                                "description": "Path to existing Topos spec file to compare against. Shows validation results and identifies new/changed elements."
                            },
                            "validate": {
                                "type": "boolean",
                                "description": "Enable validation mode (requires compare_spec). Reports invalid anchors and orphan spec elements.",
                                "default": false
                            }
                        },
                        "required": ["paths"]
                    }),
                ),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let args: Value = request
            .arguments
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let result = match request.name.as_ref() {
            "validate_spec" => self.validate_spec(&args),
            "summarize_spec" => self.summarize_spec(&args),
            "compile_context" => self.compile_context_tool(&args),
            "suggest_hole" => self.suggest_hole(&args).await,
            "extract_spec" => self.extract_spec(&args),
            _ => tool_error(format!("Unknown tool: {}", request.name)),
        };
        Ok(result)
    }
}

/// Create a Tool with all required fields.
fn make_tool(name: &str, description: &str, schema: Value) -> Tool {
    Tool {
        name: name.to_string().into(),
        description: Some(description.to_string().into()),
        input_schema: match schema {
            Value::Object(m) => Arc::new(m),
            _ => Arc::new(Map::new()),
        },
        annotations: None,
        icons: None,
        meta: None,
        output_schema: None,
        title: None,
    }
}

/// Run the MCP server on stdin/stdout.
pub async fn run_server() -> anyhow::Result<()> {
    tracing::info!("Starting Topos MCP server");

    let server = ToposServer::new();
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_info() {
        let server = ToposServer::new();
        let info = server.get_info();
        assert_eq!(&*info.server_info.name, "topos-mcp");
    }

    #[test]
    fn test_validate_spec_missing_arg() {
        let server = ToposServer::new();
        let result = server.validate_spec(&json!({}));
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_make_tool() {
        let tool = make_tool("test", "A test tool", json!({"type": "object"}));
        assert_eq!(&*tool.name, "test");
        assert!(tool.description.is_some());
    }

    #[test]
    fn test_extract_spec_missing_paths() {
        let server = ToposServer::new();
        let result = server.extract_spec(&json!({}));
        assert_eq!(result.is_error, Some(true));
        assert!(result.content[0].as_text().unwrap().text.contains("Missing 'paths'"));
    }

    #[test]
    fn test_extract_spec_with_filter() {
        use std::io::Write;
        let server = ToposServer::new();

        // Create a temp file with anchors
        let mut temp_file = tempfile::Builder::new()
            .suffix(".rs")
            .tempfile()
            .unwrap();
        writeln!(
            temp_file,
            r#"// @topos(concept="User")
pub struct User {{}}

// @topos(concept="Order")
pub struct Order {{}}

// @topos(behavior="create_order", implements="REQ-1")
pub fn create_order() {{}}"#
        )
        .unwrap();

        let path = temp_file.path().to_str().unwrap();

        // Test filter by kind=concept (should exclude behavior)
        let result = server.extract_spec(&json!({
            "paths": path,
            "filter_kind": "concept"
        }));
        assert_eq!(result.is_error, Some(false));
        let text = &result.content[0].as_text().unwrap().text;
        assert!(text.contains("User"));
        assert!(text.contains("Order"));
        assert!(!text.contains("create_order"));

        // Test filter by concept name
        let result = server.extract_spec(&json!({
            "paths": path,
            "filter_concept": "User"
        }));
        assert_eq!(result.is_error, Some(false));
        let text = &result.content[0].as_text().unwrap().text;
        assert!(text.contains("User"));
        assert!(!text.contains("Order"));
    }

    #[test]
    fn test_filter_anchors() {
        let server = ToposServer::new();

        // Create a collection with various anchors
        let mut collection = topos_analysis::AnchorCollection::new();

        // Add a concept anchor
        let mut attrs1 = std::collections::HashMap::new();
        attrs1.insert("concept".to_string(), "User".to_string());
        collection.add(topos_analysis::Anchor {
            kind: topos_analysis::AnchorKind::Concept,
            attributes: attrs1,
            file_path: "test.rs".to_string(),
            line: 0,
            code_element: None,
        });

        // Add a behavior anchor
        let mut attrs2 = std::collections::HashMap::new();
        attrs2.insert("behavior".to_string(), "create_user".to_string());
        attrs2.insert("implements".to_string(), "REQ-1".to_string());
        collection.add(topos_analysis::Anchor {
            kind: topos_analysis::AnchorKind::Behavior,
            attributes: attrs2,
            file_path: "test.rs".to_string(),
            line: 5,
            code_element: None,
        });

        // Filter by kind
        let filtered = server.filter_anchors(collection.clone(), Some("concept"), None, None, None);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.concept("User").is_some());

        // Filter by requirement
        let filtered = server.filter_anchors(collection.clone(), None, None, Some("REQ-1"), None);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.behavior("create_user").is_some());
    }
}
