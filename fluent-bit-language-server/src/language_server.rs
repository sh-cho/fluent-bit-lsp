use std::{collections::HashMap, str::FromStr};

use ropey::Rope;
use tokio::sync::RwLock;
use tower_lsp::{
    jsonrpc::Result as JsonRpcResult,
    lsp_types::{
        CompletionItem, CompletionOptions, CompletionOptionsCompletionItem, CompletionParams,
        CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
        TextDocumentContentChangeEvent, TextDocumentPositionParams, TextDocumentSyncCapability,
        TextDocumentSyncKind, Url,
    },
    Client, LanguageServer,
};
use tree_sitter::{Node, Point};

use crate::{
    completion::{get_completion, get_hover_info, SectionType},
    document::{PositionEncodingKind, TextDocument},
};

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

    /// TODO: use TreeCursor?
    pub async fn get_section_type_at_point(&self, url: &Url, point: &Point) -> Option<SectionType> {
        let r = self.map.read().await;
        let TextDocument { rope, tree, .. } = r.get(&url)?;
        let Some(tree) = tree else {
            // could this happen?
            return None;
        };

        let node = tree
            .root_node()
            .descendant_for_point_range(*point, *point)?;

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

        match node.kind() {
            "section_body" => {
                if let Some(parent) = node.parent() {
                    Self::get_section_name(&parent, &rope)
                        .and_then(|name| SectionType::from_str(&name).ok())
                } else {
                    None
                }
            }
            "key_type" => {
                // should go up parent tree until it finds section node
                let mut parent = node.parent();
                while let Some(p) = parent {
                    if let Some(section_name) = Self::get_section_name(&p, &rope) {
                        return SectionType::from_str(&section_name).ok();
                    }
                    parent = p.parent();
                }
                None
            }
            _ => None,
        }
    }

    fn get_section_name(node: &Node, rope: &Rope) -> Option<String> {
        if node.kind() == "section" {
            if let Some(section_name_node) = node
                .child_by_field_name("header")
                .and_then(|n| n.child_by_field_name("name"))
            {
                let byte_range = section_name_node.byte_range();
                let section_name = rope.slice(byte_range).as_str().unwrap();
                return Some(section_name.to_string());
            }
        }
        None
    }

    pub async fn get_key_at_point(&self, url: &Url, point: &Point) -> Option<String> {
        let r = self.map.read().await;
        let TextDocument { rope, tree, .. } = r.get(&url)?;
        let Some(tree) = tree else {
            return None;
        };
        let node = tree
            .root_node()
            .descendant_for_point_range(*point, *point)?;

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

        if node.kind() == "key_type" {
            let byte_range = node.byte_range();
            let key = rope.slice(byte_range).as_str().unwrap();
            return Some(key.to_string());
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
                hover_provider: Some(HoverProviderCapability::Simple(true)),
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
            ret.extend(get_completion(&section));
        } else {
            return Ok(None);
        }

        Ok(Some(CompletionResponse::Array(ret)))
    }

    async fn hover(&self, params: HoverParams) -> JsonRpcResult<Option<Hover>> {
        let TextDocumentPositionParams {
            text_document,
            position,
        } = params.text_document_position_params;

        let point = Point {
            row: position.line as usize,
            column: position.character as usize,
        };
        let Some(key) = self.get_key_at_point(&text_document.uri, &point).await else {
            return Ok(None);
        };
        let Some(section_type) = self
            .get_section_type_at_point(&text_document.uri, &point)
            .await
        else {
            return Ok(None);
        };

        let Some(param_info) = get_hover_info(&section_type, &key) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(param_info.into()),
            range: None,
        }))
    }
}
