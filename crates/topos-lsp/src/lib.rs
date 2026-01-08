//! Topos Language Server Protocol implementation.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// The Topos LSP server.
#[derive(Debug)]
pub struct ToposServer {
    pub client: Client,
}

impl ToposServer {
    /// Validate document text and publish diagnostics.
    async fn validate_text(&self, uri: Url, text: &str) {
        let analysis_diagnostics = topos_analysis::check(text);

        let diagnostics: Vec<Diagnostic> = analysis_diagnostics
            .into_iter()
            .map(|d| Diagnostic {
                range: Range {
                    start: Position::new(d.line, d.column),
                    end: Position::new(d.end_line, d.end_column),
                },
                severity: Some(match d.severity {
                    topos_analysis::Severity::Error => DiagnosticSeverity::ERROR,
                    topos_analysis::Severity::Warning => DiagnosticSeverity::WARNING,
                    topos_analysis::Severity::Info => DiagnosticSeverity::INFORMATION,
                }),
                source: Some("topos".to_string()),
                message: d.message,
                ..Diagnostic::default()
            })
            .collect();

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
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
        self.validate_text(params.text_document.uri, &params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.validate_text(params.text_document.uri, &change.text).await;
        }
    }
}

pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| ToposServer { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::LspService;

    #[tokio::test]
    async fn test_initialize() {
        let (service, _) = LspService::new(|client| ToposServer { client });
        
        let params = InitializeParams {
            ..Default::default()
        };
        
        let response = service.inner().initialize(params).await.unwrap();
        
        assert_eq!(response.server_info.as_ref().unwrap().name, "topos-lsp");
    }
}
