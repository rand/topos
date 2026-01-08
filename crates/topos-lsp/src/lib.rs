//! Topos Language Server Protocol implementation.
//!
//! Provides IDE features for Topos specification files:
//! - Diagnostics (syntax and semantic errors)
//! - Hover information
//! - Go-to-definition
//! - Find references
//! - Completions

use dashmap::DashMap;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use topos_analysis::{
    compute_diagnostics, compute_symbols, compute_traceability, resolve_references,
    AnalysisDatabase, ReferenceKind, SymbolKind,
};
use topos_syntax::Span;

/// Document state for a single file.
struct DocumentState {
    /// The source text.
    text: String,
    /// The analysis database file handle.
    file: topos_analysis::SourceFile,
}

/// The Topos LSP server.
pub struct ToposServer {
    /// LSP client for sending notifications.
    pub client: Client,
    /// Analysis database (shared across all documents).
    db: Mutex<AnalysisDatabase>,
    /// Document states keyed by URI.
    documents: DashMap<Url, DocumentState>,
}

impl ToposServer {
    /// Create a new server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            db: Mutex::new(AnalysisDatabase::new()),
            documents: DashMap::new(),
        }
    }

    /// Open or update a document.
    async fn update_document(&self, uri: &Url, text: String) {
        let path = uri.path().to_string();
        let mut db = self.db.lock().await;

        if let Some(mut state) = self.documents.get_mut(uri) {
            // Update existing document
            db.update_file(state.file, text.clone());
            state.text = text;
        } else {
            // Add new document
            let file = db.add_file(path, text.clone());
            self.documents.insert(uri.clone(), DocumentState { text, file });
        }
    }

    /// Get the file handle for a document.
    fn get_file(&self, uri: &Url) -> Option<topos_analysis::SourceFile> {
        self.documents.get(uri).map(|state| state.file)
    }

    /// Validate document and publish diagnostics.
    async fn publish_diagnostics(&self, uri: Url) {
        let Some(file) = self.get_file(&uri) else {
            return;
        };

        let db = self.db.lock().await;
        let semantic_diags = compute_diagnostics(&*db, file);

        let lsp_diagnostics: Vec<Diagnostic> = semantic_diags
            .diagnostics
            .iter()
            .map(|d| {
                let range = span_to_range(&d.span);
                Diagnostic {
                    range,
                    severity: Some(match d.severity {
                        topos_analysis::SemanticSeverity::Error => DiagnosticSeverity::ERROR,
                        topos_analysis::SemanticSeverity::Warning => DiagnosticSeverity::WARNING,
                        topos_analysis::SemanticSeverity::Hint => DiagnosticSeverity::HINT,
                    }),
                    source: Some("topos".to_string()),
                    message: d.message.clone(),
                    ..Diagnostic::default()
                }
            })
            .collect();

        self.client
            .publish_diagnostics(uri, lsp_diagnostics, None)
            .await;
    }

    /// Get hover information at a position.
    async fn hover_at(&self, uri: &Url, position: Position) -> Option<Hover> {
        let file = self.get_file(uri)?;
        let db = self.db.lock().await;

        let symbol_table = compute_symbols(&*db, file);
        let offset = self.position_to_offset(uri, position)?;

        // Find symbol at position
        for symbol in symbol_table.symbols.values() {
            if span_contains(&symbol.span, offset) {
                let kind_str = match symbol.kind {
                    SymbolKind::Requirement => "requirement",
                    SymbolKind::Task => "task",
                    SymbolKind::Concept => "concept",
                    SymbolKind::Behavior => "behavior",
                    SymbolKind::Invariant => "invariant",
                    SymbolKind::Field => "field",
                };

                let contents = format!("**{}** `{}`", kind_str, symbol.name);
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(span_to_range(&symbol.span)),
                });
            }
        }

        // Check references
        let resolved = resolve_references(&*db, file);
        for ref_result in &resolved.references {
            if span_contains(&ref_result.reference.span, offset) {
                let kind_str = match ref_result.reference.kind {
                    ReferenceKind::Type => "type reference",
                    ReferenceKind::Requirement => "requirement reference",
                    ReferenceKind::Task => "task reference",
                    ReferenceKind::Concept => "concept reference",
                };

                let status = if ref_result.symbol.is_some() {
                    "✓ resolved"
                } else {
                    "✗ unresolved"
                };

                let contents = format!(
                    "**{}** `{}`\n\n{}",
                    kind_str, ref_result.reference.name, status
                );
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(span_to_range(&ref_result.reference.span)),
                });
            }
        }

        None
    }

    /// Go to definition at a position.
    async fn goto_definition(&self, uri: &Url, position: Position) -> Option<Location> {
        let file = self.get_file(uri)?;
        let db = self.db.lock().await;

        let offset = self.position_to_offset(uri, position)?;
        let resolved = resolve_references(&*db, file);

        // Find reference at position
        for ref_result in &resolved.references {
            if span_contains(&ref_result.reference.span, offset)
                && let Some(symbol) = &ref_result.symbol {
                    return Some(Location {
                        uri: uri.clone(),
                        range: span_to_range(&symbol.span),
                    });
                }
        }

        None
    }

    /// Find all references to symbol at position.
    async fn find_references(&self, uri: &Url, position: Position) -> Vec<Location> {
        let Some(file) = self.get_file(uri) else {
            return vec![];
        };
        let db = self.db.lock().await;

        let offset = self.position_to_offset(uri, position).unwrap_or(0);
        let symbol_table = compute_symbols(&*db, file);

        // Find which symbol we're on
        let mut target_name: Option<String> = None;

        for symbol in symbol_table.symbols.values() {
            if span_contains(&symbol.span, offset) {
                target_name = Some(symbol.name.clone());
                break;
            }
        }

        // Also check if we're on a reference
        if target_name.is_none() {
            let resolved = resolve_references(&*db, file);
            for ref_result in &resolved.references {
                if span_contains(&ref_result.reference.span, offset) {
                    target_name = Some(ref_result.reference.name.clone());
                    break;
                }
            }
        }

        let Some(name) = target_name else {
            return vec![];
        };

        // Collect all locations where this name appears
        let mut locations = vec![];

        // Add definition
        if let Some(symbol) = symbol_table.get(&name) {
            locations.push(Location {
                uri: uri.clone(),
                range: span_to_range(&symbol.span),
            });
        }

        // Add all references
        let resolved = resolve_references(&*db, file);
        for ref_result in &resolved.references {
            if ref_result.reference.name == name {
                locations.push(Location {
                    uri: uri.clone(),
                    range: span_to_range(&ref_result.reference.span),
                });
            }
        }

        locations
    }

    /// Get completions at position.
    async fn completions(&self, uri: &Url, _position: Position) -> Vec<CompletionItem> {
        let Some(file) = self.get_file(uri) else {
            return vec![];
        };
        let db = self.db.lock().await;

        let symbol_table = compute_symbols(&*db, file);
        let trace = compute_traceability(&*db, file);

        let mut items = vec![];

        // Add requirements
        for id in symbol_table.requirements.keys() {
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Requirement".to_string()),
                ..Default::default()
            });
        }

        // Add tasks
        for id in symbol_table.tasks.keys() {
            let reqs: Vec<_> = trace.reqs_for_task(id).collect();
            let detail = if reqs.is_empty() {
                "Task".to_string()
            } else {
                format!("Task → {}", reqs.join(", "))
            };
            items.push(CompletionItem {
                label: id.clone(),
                kind: Some(CompletionItemKind::EVENT),
                detail: Some(detail),
                ..Default::default()
            });
        }

        // Add concepts
        for name in symbol_table.concepts.keys() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("Concept".to_string()),
                ..Default::default()
            });
        }

        // Add behaviors
        for name in symbol_table.behaviors.keys() {
            let reqs: Vec<_> = trace.reqs_for_behavior(name).collect();
            let detail = if reqs.is_empty() {
                "Behavior".to_string()
            } else {
                format!("Behavior implements {}", reqs.join(", "))
            };
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(detail),
                ..Default::default()
            });
        }

        items
    }

    /// Convert LSP position to byte offset.
    fn position_to_offset(&self, uri: &Url, position: Position) -> Option<usize> {
        let state = self.documents.get(uri)?;
        let mut offset = 0;
        let mut line = 0;

        for ch in state.text.chars() {
            if line == position.line {
                for (col, ch) in state.text[offset..].chars().enumerate() {
                    if col as u32 == position.character {
                        return Some(offset);
                    }
                    if ch == '\n' {
                        break;
                    }
                    offset += ch.len_utf8();
                }
                return Some(offset);
            }
            if ch == '\n' {
                line += 1;
            }
            offset += ch.len_utf8();
        }

        Some(offset)
    }
}

/// Convert a Span to an LSP Range.
fn span_to_range(span: &Span) -> Range {
    Range {
        start: Position::new(span.start_line, span.start_col),
        end: Position::new(span.end_line, span.end_col),
    }
}

/// Check if a span contains an offset.
fn span_contains(span: &Span, offset: usize) -> bool {
    offset >= span.start as usize && offset < span.end as usize
}

#[tower_lsp::async_trait]
impl LanguageServer for ToposServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "topos-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "[".to_string(),
                        "`".to_string(),
                        "-".to_string(),
                    ]),
                    ..Default::default()
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Topos LSP server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update_document(&params.text_document.uri, params.text_document.text)
            .await;
        self.publish_diagnostics(params.text_document.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.update_document(&params.text_document.uri, change.text.clone())
                .await;
            self.publish_diagnostics(params.text_document.uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        Ok(self.hover_at(&uri, position).await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        Ok(self
            .goto_definition(&uri, position)
            .await
            .map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let locations = self.find_references(&uri, position).await;
        Ok(if locations.is_empty() {
            None
        } else {
            Some(locations)
        })
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let items = self.completions(&uri, position).await;
        Ok(if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        })
    }
}

/// Run the LSP server on stdin/stdout.
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(ToposServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::LspService;

    #[tokio::test]
    async fn test_initialize() {
        let (service, _) = LspService::new(ToposServer::new);

        let params = InitializeParams {
            ..Default::default()
        };

        let response = service.inner().initialize(params).await.unwrap();

        assert_eq!(response.server_info.as_ref().unwrap().name, "topos-lsp");
        assert!(response.capabilities.hover_provider.is_some());
        assert!(response.capabilities.definition_provider.is_some());
        assert!(response.capabilities.references_provider.is_some());
        assert!(response.capabilities.completion_provider.is_some());
    }

    #[tokio::test]
    async fn test_document_lifecycle() {
        let (service, _) = LspService::new(ToposServer::new);
        let server = service.inner();

        // Initialize
        server
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        // Open document
        let uri = Url::parse("file:///test.tps").unwrap();
        let text = "spec Test\n\n# Requirements\n\n## REQ-1: Test\nDescription.\n".to_string();

        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "topos".to_string(),
                    version: 1,
                    text,
                },
            })
            .await;

        // Verify document is tracked
        assert!(server.documents.contains_key(&uri));
    }

    #[tokio::test]
    async fn test_completions() {
        let (service, _) = LspService::new(ToposServer::new);
        let server = service.inner();

        server
            .initialize(InitializeParams::default())
            .await
            .unwrap();

        let uri = Url::parse("file:///test.tps").unwrap();
        let text = r#"spec Test

# Requirements

## REQ-1: First Requirement
Description.

# Concepts

Concept User:
  field name (`String`)
"#
        .to_string();

        server
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "topos".to_string(),
                    version: 1,
                    text,
                },
            })
            .await;

        let completions = server.completions(&uri, Position::new(0, 0)).await;
        assert!(!completions.is_empty());

        let labels: Vec<_> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"REQ-1"));
        assert!(labels.contains(&"User"));
    }
}
