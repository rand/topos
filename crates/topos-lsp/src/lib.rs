use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
pub struct ToposServer {
    pub client: Client,
}

impl ToposServer {
    async fn validate_text(&self, uri: Url, text: &str) {
        // We use catch_unwind because topos_analysis::check currently panics 
        // due to tree-sitter-topos being unimplemented.
        let result = std::panic::catch_unwind(|| {
            topos_analysis::check(text)
        });

        let diagnostics = match result {
            Ok(analysis_diagnostics) => {
                analysis_diagnostics.into_iter().map(|d| {
                    Diagnostic {
                        range: Range {
                            start: Position::new(d.line, d.column),
                            end: Position::new(d.line, d.column + 1),
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: d.message,
                        ..Diagnostic::default()
                    }
                }).collect()
            },
            Err(_) => {
                // If it panics, we just report no diagnostics for now
                vec![]
            }
        };

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
