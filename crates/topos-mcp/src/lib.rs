//! MCP (Model Context Protocol) server and client for Topos.
//!
//! ## Server
//!
//! Provides AI-accessible tools for working with Topos specifications:
//! - `validate_spec` - Validate a spec file and return diagnostics
//! - `summarize_spec` - Get an AI-friendly summary of a spec
//! - `compile_context` - Compile task-focused context
//! - `suggest_hole` - Get LLM-powered suggestions for typed holes
//!
//! ## Client
//!
//! Provides MCP client for LLM-powered semantic analysis:
//! - Semantic drift detection between spec versions
//! - LLM-as-Judge for prose requirement verification

pub mod client;

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
    fn suggest_hole(&self, args: &Value) -> CallToolResult {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return tool_error("Error: Missing 'path' argument"),
        };

        // Position can be specified by line/column or offset
        let line = args.get("line").and_then(|v| v.as_u64()).map(|v| v as u32);
        let column = args.get("column").and_then(|v| v.as_u64()).map(|v| v as u32);
        let offset = args.get("offset").and_then(|v| v.as_u64()).map(|v| v as u32);

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

        // Build context for suggestions
        let mut output = String::new();
        output.push_str("# Typed Hole Suggestions\n\n");
        output.push_str(&format!("## Hole Context\n\n{}\n", hole.prompt_context()));

        // Get surrounding context from the spec
        output.push_str("## Surrounding Code\n\n```topos\n");
        let span = hole.span();
        let lines: Vec<&str> = content.lines().collect();
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

        // Generate suggestions based on context
        output.push_str("## Suggestions\n\n");

        // Provide type-aware suggestions
        if let Some(ref type_hint) = hole.type_hint {
            output.push_str(&format!(
                "Based on the type constraint `{}`, consider:\n\n",
                type_hint
            ));
            output.push_str(&format!("1. `{}` - Use the specified type directly\n", type_hint));
        } else {
            output.push_str("No type constraint specified. Consider:\n\n");
        }

        // Add context-aware suggestions based on parent
        match &hole.parent {
            topos_analysis::HoleParent::ConceptField { concept_name, field_name } => {
                output.push_str(&format!(
                    "\nFor field `{}` in concept `{}`:\n",
                    field_name, concept_name
                ));
                // Common field type patterns
                if field_name.contains("id") {
                    output.push_str("- `String` or `UUID` - Common ID types\n");
                }
                if field_name.contains("date") || field_name.contains("time") {
                    output.push_str("- `DateTime` or `Timestamp` - For temporal data\n");
                }
                if field_name.contains("status") || field_name.contains("state") {
                    output.push_str("- `Enum` type - Consider defining a status enum\n");
                }
                if field_name.contains("amount") || field_name.contains("price") || field_name.contains("total") {
                    output.push_str("- `Currency` or `Decimal` - For monetary values\n");
                }
            }
            topos_analysis::HoleParent::BehaviorSignature { behavior_name, position } => {
                output.push_str(&format!(
                    "\nFor {} of behavior `{}`:\n",
                    position.description().to_lowercase(),
                    behavior_name
                ));
                output.push_str("- Consider what types flow through this behavior\n");
            }
            topos_analysis::HoleParent::BehaviorReturns { behavior_name } => {
                output.push_str(&format!(
                    "\nFor returns clause of behavior `{}`:\n",
                    behavior_name
                ));
                output.push_str("- `Result<T, E>` - For fallible operations\n");
                output.push_str("- `Option<T>` - For nullable returns\n");
            }
            topos_analysis::HoleParent::BehaviorConstraint { behavior_name, constraint_kind } => {
                output.push_str(&format!(
                    "\nFor {} constraint of behavior `{}`:\n",
                    constraint_kind, behavior_name
                ));
                output.push_str("- Consider what invariants should hold\n");
            }
            topos_analysis::HoleParent::Unknown => {}
        }

        // Related concepts
        if !hole.related_concepts.is_empty() {
            output.push_str("\n## Related Concepts\n\n");
            output.push_str("These concepts are used nearby and might be relevant:\n\n");
            for concept in &hole.related_concepts {
                output.push_str(&format!("- `{}`\n", concept));
            }
        }

        tool_result(output)
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
                    "Get suggestions for filling a typed hole ([?]) in a Topos specification, with context-aware type recommendations",
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
                            }
                        },
                        "required": ["path"]
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
            "suggest_hole" => self.suggest_hole(&args),
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
}
