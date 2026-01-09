//! MCP (Model Context Protocol) server and client for Topos.
//!
//! ## Server
//!
//! Provides AI-accessible tools for working with Topos specifications:
//! - `validate_spec` - Validate a spec file and return diagnostics
//! - `summarize_spec` - Get an AI-friendly summary of a spec
//! - `compile_context` - Compile task-focused context
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
