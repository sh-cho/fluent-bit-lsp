use std::collections::HashMap;
use std::str::FromStr;

use tokio::sync::RwLock;
use tower_lsp::{Client, LanguageServer};
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionOptions, CompletionOptionsCompletionItem, CompletionParams,
    CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializedParams, InitializeParams, InitializeResult, MessageType,
    ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentPositionParams,
    TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tree_sitter::Point;

use crate::completion::{INPUT_COMPLETIONS, OUTPUT_COMPLETIONS};
use crate::document::{PositionEncodingKind, TextDocument};
use crate::SectionType;

pub struct Backend {
    pub(crate) client: Client,
    pub(crate) map: RwLock<HashMap<Url, TextDocument>>,
}

impl Backend {
    pub async fn open_file(&self, url: &Url, source_code: &str) {
        let mut wr = self.map.write().await;
        wr.insert(url.clone(), TextDocument::new(source_code));
    }

    pub async fn update_file(&self, url: &Url, change: &TextDocumentContentChangeEvent) {
        let mut wr = self.map.write().await;
        if let Some(document) = wr.get_mut(url) {
            document
                .apply_content_change(change, PositionEncodingKind::UTF16)
                .unwrap();
        }
    }

    /// TODO: use TreeCursor
    pub async fn get_section_type_at_point(&self, url: &Url, point: &Point) -> Option<SectionType> {
        let r = self.map.read().await;
        let Some(TextDocument { rope, tree, .. }) = r.get(&url) else {
            return None;
        };
        let Some(tree) = tree else {
            // could this happen?
            return None;
        };

        let Some(node) = tree.root_node().descendant_for_point_range(*point, *point) else {
            return None;
        };

        self.client
            .log_message(
                MessageType::INFO,
                format!(
                    "node.kind: {:?} / node: {:?} / point: {:?}",
                    node.kind(),
                    node.clone(),
                    point
                ),
            )
            .await;

        if node.kind() == "section_body" {
            if let Some(parent) = node.parent() {
                if parent.kind() == "section" {
                    if let Some(section_name_node) = parent
                        .child_by_field_name("header")
                        .and_then(|n| n.child_by_field_name("name"))
                    {
                        let byte_range = section_name_node.byte_range();
                        let section_name = rope.slice(byte_range).as_str().unwrap();
                        return SectionType::from_str(section_name).ok();
                    }
                }
            }
        }

        None
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> JsonRpcResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    // TextDocumentSyncKind::FULL, // TODO: incremental
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: Some(CompletionOptionsCompletionItem {
                        label_details_support: Some(true),
                    }),
                }),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "fluent-bit language server initialized")
            .await;
    }

    async fn shutdown(&self) -> JsonRpcResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("file opened / {}", params.text_document.uri),
            )
            .await;

        let url = params.text_document.uri;
        let source_code = params.text_document.text.as_str();

        self.open_file(&url, source_code).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("did_change: {}", params.text_document.uri),
            )
            .await;

        let url = params.text_document.uri;

        for c in params.content_changes {
            // assume only changes
            if let Some(range) = c.range {
                self.client
                    .log_message(MessageType::INFO, format!("range: {:?}", range))
                    .await;

                self.update_file(&url, &c).await;
            } else {
                self.client
                    .log_message(MessageType::INFO, format!("full text change"))
                    .await;
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("did_close: {}", params.text_document.uri),
            )
            .await;

        let url = params.text_document.uri;
        // self.map.borrow_mut()
        //     .remove(&url);

        self.map.write().await.remove(&url);
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> JsonRpcResult<Option<CompletionResponse>> {
        let TextDocumentPositionParams {
            text_document,
            position,
        } = params.text_document_position;

        let point = Point {
            row: position.line as usize,
            column: position.character as usize,
        };

        // TEMP
        let section_type = self
            .get_section_type_at_point(&text_document.uri, &point)
            .await;
        let mut ret: Vec<CompletionItem> = Vec::new();

        self.client
            .log_message(
                MessageType::INFO,
                format!("section_type: {:?}", section_type),
            )
            .await;

        if let Some(section) = section_type {
            match section {
                SectionType::Input => {
                    ret.push(CompletionItem::new_simple(
                        "InputLabel".to_string(),
                        "InputDetail".to_string(),
                    ));
                    ret.extend(INPUT_COMPLETIONS.iter().cloned().map(CompletionItem::from));
                }
                SectionType::Parser => {
                    ret.push(CompletionItem::new_simple(
                        "ParserLabel".to_string(),
                        "ParserDetail".to_string(),
                    ));
                }
                SectionType::Filter => {
                    ret.push(CompletionItem::new_simple(
                        "FilterLabel".to_string(),
                        "FilterDetail".to_string(),
                    ));
                }
                SectionType::Output => {
                    ret.push(CompletionItem::new_simple(
                        "OutputLabel".to_string(),
                        "OutputDetail".to_string(),
                    ));
                    ret.extend(OUTPUT_COMPLETIONS.iter().cloned().map(CompletionItem::from));
                }
                SectionType::Other(_) => {
                    ret.push(CompletionItem::new_simple(
                        "OtherLabel".to_string(),
                        "OtherDetail".to_string(),
                    ));
                }
            }
        } else {
            return Ok(None);
        }

        Ok(Some(CompletionResponse::Array(ret)))
    }
}
