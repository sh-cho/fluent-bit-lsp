use std::future::Future;
use std::pin::Pin;
use dashmap::DashMap;
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tree_sitter::{Node, Tree};
use tree_sitter_traversal::{Order, traverse};
use crate::parser::parse;
use crate::semantic_token::{token_modifiers, token_types};

mod parser;
mod semantic_token;

#[derive(Debug)]
struct Backend {
    client: Client,
    // document_map: DashMap<String, Rope>,
    tree: Option<Tree>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: String::from("fluent-bit-lsp"),
                version: None,
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["dummy.do_something".to_string()],
                    work_done_progress_options: Default::default(),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: None,
                            },
                            legend: SemanticTokensLegend {
                                token_types: token_types(),
                                token_modifiers: token_modifiers(),
                            },
                            // range: Some(true),
                            // full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
                            range: None,
                            full: Some(SemanticTokensFullOptions::Bool(true))
                        },
                    ),
                ),
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                ..ServerCapabilities::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;

        self.on_change(params.text_document)
            .await;
    }

    async fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file changed!")
            .await;
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
            language_id: "".to_string(),  // TODO: ?
        })
            .await;
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        self.client
            .log_message(MessageType::INFO, "semantic_token_full")
            .await;

        if let Some(tree) = &self.tree {

            let preorder: Vec<Node<'_>> = traverse(tree.walk(), Order::Pre).collect::<Vec<_>>();
            let semantic_tokens = preorder.iter()
                .filter_map(|node| {
                    let range = Range {
                        start: Position {
                            line: node.start_position().row as u32,
                            character: node.start_position().column as u32,
                        },
                        end: Position {
                            line: node.end_position().row as u32,
                            character: node.end_position().column as u32,
                        },
                    };
                    // TODO
                    let token_type = token_types().iter().position(|t| t == &node.kind_id().to_string());
                    let token_type = token_type.map(|t| t as u32);
                    token_type.map(|token_type| SemanticToken {
                        delta_line: range.start.line,
                        delta_start: range.start.character,
                        length: range.end.character - range.start.character,
                        token_type,
                        token_modifiers_bitset: 0,
                    })
                })
                .collect::<Vec<_>>();

            Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_tokens,
            })))
        } else {
            Ok(None)
        }
    }

    fn semantic_tokens_full_delta<'life0, 'async_trait>(&'life0 self, params: SemanticTokensDeltaParams) -> Pin<Box<dyn Future<Output=Result<Option<SemanticTokensFullDeltaResult>>> + Send + 'async_trait>> where 'life0: 'async_trait, Self: 'async_trait {
        todo!()
    }

    fn semantic_tokens_range<'life0, 'async_trait>(&'life0 self, params: SemanticTokensRangeParams) -> Pin<Box<dyn Future<Output=Result<Option<SemanticTokensRangeResult>>> + Send + 'async_trait>> where 'life0: 'async_trait, Self: 'async_trait {
        todo!()
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

        match self.client.apply_edit(WorkspaceEdit::default()).await {
            Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
            Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
            Err(err) => self.client.log_message(MessageType::ERROR, err).await,
        }

        Ok(None)
    }
}

impl Backend {
    async fn on_change(&mut self, params: TextDocumentItem) {
        let rope = Rope::from_str(&params.text);
        // self.document_map
        //     .insert(params.uri.to_string(), rope.clone());

        let new_tree = parse(*rope, *self.tree);
        self.tree = Option::from(new_tree);


    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend {
        client,
        // document_map: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
