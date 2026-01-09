//! Topos Language Server Protocol implementation.
//!
//! Provides IDE features for Topos specification files:
//! - Diagnostics (syntax and semantic errors)
//! - Hover information
//! - Go-to-definition
//! - Find references
//! - Completions
//! - Code actions (fill typed holes with suggestions)

use dashmap::DashMap;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use topos_analysis::{
    compute_diagnostics, compute_traceability, compute_unified_symbols, extract_holes,
    resolve_references, AnalysisDatabase, ForeignSymbolKind, HoleParent, ReferenceKind,
    UnifiedSymbol,
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

        let unified_table = compute_unified_symbols(&*db, file);
        let offset = self.position_to_offset(uri, position)?;

        // Find symbol at position (Topos symbols)
        for symbol in unified_table.topos().symbols.values() {
            if span_contains(&symbol.span, offset) {
                let unified = UnifiedSymbol::Topos(symbol.clone());
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: unified.hover_docs(),
                    }),
                    range: Some(span_to_range(&symbol.span)),
                });
            }
        }

        // Check references - now resolve against unified table
        let resolved = resolve_references(&*db, file);
        for ref_result in &resolved.references {
            if span_contains(&ref_result.reference.span, offset) {
                // Try to resolve against unified symbol table (includes foreign symbols)
                if let Some(unified_sym) = unified_table.get(&ref_result.reference.name) {
                    let kind_str = match ref_result.reference.kind {
                        ReferenceKind::Type => "type reference",
                        ReferenceKind::Requirement => "requirement reference",
                        ReferenceKind::Task => "task reference",
                        ReferenceKind::Concept => "concept reference",
                    };

                    let target_info = unified_sym.hover_docs();
                    let contents = format!(
                        "**{}** `{}`\n\n✓ resolved to:\n\n{}",
                        kind_str, ref_result.reference.name, target_info
                    );
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: contents,
                        }),
                        range: Some(span_to_range(&ref_result.reference.span)),
                    });
                }

                // Unresolved reference
                let kind_str = match ref_result.reference.kind {
                    ReferenceKind::Type => "type reference",
                    ReferenceKind::Requirement => "requirement reference",
                    ReferenceKind::Task => "task reference",
                    ReferenceKind::Concept => "concept reference",
                };

                let contents = format!(
                    "**{}** `{}`\n\n✗ unresolved",
                    kind_str, ref_result.reference.name
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
        let unified_table = compute_unified_symbols(&*db, file);

        // Find which symbol we're on
        let mut target_name: Option<String> = None;

        // Check Topos symbols
        for symbol in unified_table.topos().symbols.values() {
            if span_contains(&symbol.span, offset) {
                target_name = Some(symbol.name.clone());
                break;
            }
        }

        // Check foreign symbols
        if target_name.is_none() {
            for symbol in &unified_table.foreign().symbols {
                if span_contains(&symbol.span, offset) {
                    target_name = Some(symbol.name.clone());
                    break;
                }
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

        // Add definition (from unified table)
        if let Some(unified_sym) = unified_table.get(&name) {
            locations.push(Location {
                uri: uri.clone(),
                range: span_to_range(&unified_sym.span()),
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

        let unified_table = compute_unified_symbols(&*db, file);
        let symbol_table = unified_table.topos();
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

        // Add foreign symbols (types from embedded code blocks)
        for sym in &unified_table.foreign().symbols {
            let (kind, detail) = match sym.kind {
                ForeignSymbolKind::Model => (CompletionItemKind::STRUCT, format!("{} model", sym.language)),
                ForeignSymbolKind::Interface => (CompletionItemKind::INTERFACE, format!("{} interface", sym.language)),
                ForeignSymbolKind::TypeAlias => (CompletionItemKind::TYPE_PARAMETER, format!("{} type", sym.language)),
                ForeignSymbolKind::Enum => (CompletionItemKind::ENUM, format!("{} enum", sym.language)),
                ForeignSymbolKind::Union => (CompletionItemKind::ENUM, format!("{} union", sym.language)),
                ForeignSymbolKind::Schema => (CompletionItemKind::STRUCT, format!("{} schema", sym.language)),
                ForeignSymbolKind::Namespace => (CompletionItemKind::MODULE, format!("{} namespace", sym.language)),
                ForeignSymbolKind::Operation => (CompletionItemKind::METHOD, format!("{} operation", sym.language)),
            };
            items.push(CompletionItem {
                label: sym.name.clone(),
                kind: Some(kind),
                detail: Some(detail),
                documentation: Some(Documentation::String(sym.declaration.clone())),
                ..Default::default()
            });
        }

        items
    }

    /// Get code actions at a position (e.g., fill hole suggestions).
    async fn code_actions(&self, uri: &Url, range: Range) -> Vec<CodeActionOrCommand> {
        let Some(file) = self.get_file(uri) else {
            return vec![];
        };
        let db = self.db.lock().await;

        // Extract holes from the document
        let holes = extract_holes(&*db, file);

        if holes.is_empty() {
            return vec![];
        }

        let mut actions = vec![];

        // Check if any hole overlaps with the requested range
        for hole_ctx in &holes.holes {
            let hole_range = span_to_range(&hole_ctx.span());

            // Check if ranges overlap
            if ranges_overlap(&range, &hole_range) {
                // Generate suggestions based on hole context
                let suggestions = self.generate_hole_suggestions(hole_ctx);

                for (idx, (label, new_text)) in suggestions.into_iter().enumerate() {
                    let edit = TextEdit {
                        range: hole_range,
                        new_text,
                    };

                    let mut changes = std::collections::HashMap::new();
                    changes.insert(uri.clone(), vec![edit]);

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: label,
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: None,
                        edit: Some(WorkspaceEdit {
                            changes: Some(changes),
                            document_changes: None,
                            change_annotations: None,
                        }),
                        command: None,
                        is_preferred: Some(idx == 0), // First suggestion is preferred
                        disabled: None,
                        data: None,
                    }));
                }
            }
        }

        actions
    }

    /// Generate suggestions for filling a hole based on its context.
    fn generate_hole_suggestions(
        &self,
        hole_ctx: &topos_analysis::HoleWithContext,
    ) -> Vec<(String, String)> {
        let mut suggestions = vec![];

        // If there's a type hint, suggest using it directly
        if let Some(ref type_hint) = hole_ctx.type_hint {
            suggestions.push((
                format!("Fill with type: {}", type_hint),
                format!("({})", type_hint),
            ));
        }

        // Generate context-aware suggestions based on parent
        match &hole_ctx.parent {
            HoleParent::ConceptField { field_name, .. } => {
                // Suggest based on field name patterns
                if field_name.contains("id") {
                    suggestions.push(("Fill with `String` (common ID type)".to_string(), "(`String`)".to_string()));
                    suggestions.push(("Fill with `UUID` (unique ID type)".to_string(), "(`UUID`)".to_string()));
                }
                if field_name.contains("date") || field_name.contains("time") || field_name.contains("at") {
                    suggestions.push(("Fill with `DateTime`".to_string(), "(`DateTime`)".to_string()));
                    suggestions.push(("Fill with `Timestamp`".to_string(), "(`Timestamp`)".to_string()));
                }
                if field_name.contains("status") || field_name.contains("state") {
                    suggestions.push(("Fill with `Enum` (define a status enum)".to_string(), "(`Status`)".to_string()));
                }
                if field_name.contains("amount") || field_name.contains("price") || field_name.contains("total") || field_name.contains("cost") {
                    suggestions.push(("Fill with `Currency`".to_string(), "(`Currency`)".to_string()));
                    suggestions.push(("Fill with `Decimal`".to_string(), "(`Decimal`)".to_string()));
                }
                if field_name.contains("email") {
                    suggestions.push(("Fill with `Email`".to_string(), "(`Email`)".to_string()));
                }
                if field_name.contains("name") || field_name.contains("title") || field_name.contains("description") {
                    suggestions.push(("Fill with `String`".to_string(), "(`String`)".to_string()));
                }
                if field_name.contains("count") || field_name.contains("number") || field_name.contains("quantity") {
                    suggestions.push(("Fill with `Int`".to_string(), "(`Int`)".to_string()));
                }
                if field_name.contains("enabled") || field_name.contains("active") || field_name.contains("is_") {
                    suggestions.push(("Fill with `Bool`".to_string(), "(`Bool`)".to_string()));
                }
            }
            HoleParent::BehaviorSignature { .. } => {
                suggestions.push(("Fill with input type".to_string(), "(`Input`)".to_string()));
            }
            HoleParent::BehaviorReturns { .. } => {
                suggestions.push(("Fill with `Result<T, E>`".to_string(), "(`Result<Success, Error>`)".to_string()));
                suggestions.push(("Fill with `Option<T>`".to_string(), "(`Option<Value>`)".to_string()));
            }
            HoleParent::BehaviorConstraint { .. } => {
                suggestions.push(("Fill with constraint expression".to_string(), "(value > 0)".to_string()));
            }
            HoleParent::Unknown => {}
        }

        // If we have related concepts, suggest them
        for concept in &hole_ctx.related_concepts {
            suggestions.push((
                format!("Fill with related concept: `{}`", concept),
                format!("(`{}`)", concept),
            ));
        }

        // Default suggestions if nothing else matched
        if suggestions.is_empty() {
            suggestions.push(("Fill with `String`".to_string(), "(`String`)".to_string()));
            suggestions.push(("Fill with `Int`".to_string(), "(`Int`)".to_string()));
            suggestions.push(("Fill with `Bool`".to_string(), "(`Bool`)".to_string()));
        }

        suggestions
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

/// Check if two LSP ranges overlap.
fn ranges_overlap(a: &Range, b: &Range) -> bool {
    // Ranges overlap if neither is entirely before the other
    !(a.end.line < b.start.line
        || (a.end.line == b.start.line && a.end.character < b.start.character)
        || b.end.line < a.start.line
        || (b.end.line == a.start.line && b.end.character < a.start.character))
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..Default::default()
                    },
                )),
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

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let actions = self.code_actions(&uri, range).await;
        Ok(if actions.is_empty() {
            None
        } else {
            Some(actions)
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
