use std::collections::HashMap;
use std::str::FromStr;

use tokio::sync::RwLock;
use tower_lsp::{Client, LanguageServer};
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionOptions, CompletionOptionsCompletionItem, CompletionParams,
    CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializedParams, InitializeParams, InitializeResult, MessageType,
    ServerCapabilities, TextDocumentPositionParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use tree_sitter::Point;
use crate::completion::INPUT_COMPLETIONS;

use crate::parser::MyParser;
use crate::SectionType;

pub struct Backend {
    pub(crate) client: Client,
    pub(crate) map: RwLock<HashMap<Url, MyParser>>,
}

impl Backend {
    pub async fn open_or_update(&self, url: Url, source_code: &str) {
        let mut wr = self.map.write().await;

        // match wr.get_mut(&url) {
        //     Some(my_parser) => {
        //         my_parser.update(source_code);
        //     }
        //     None => {
        //         wr.insert(url, MyParser::new(source_code));
        //     }
        // }

        // TODO: update
        wr.insert(url, MyParser::new(source_code));
    }

    /// TODO: use TreeCursor
    pub async fn get_section_type_at_point(&self, url: Url, point: &Point) -> Option<SectionType> {
        let r = self.map.read().await;
        let Some(MyParser {
            parser,
            tree,
            source_code,
        }) = r.get(&url)
        else {
            return None;
        };

        let Some(mut node) = tree.root_node().descendant_for_point_range(*point, *point) else {
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
            self.client
                .log_message(MessageType::INFO, format!("1"))
                .await;
            if let Some(parent) = node.parent() {
                self.client
                    .log_message(MessageType::INFO, format!("2"))
                    .await;
                if parent.kind() == "section" {
                    self.client
                        .log_message(MessageType::INFO, format!("3"))
                        .await;

                    if let Some(section_name) = parent
                        .child_by_field_name("header")
                        .and_then(|n| n.child_by_field_name("name"))
                        .map(|n| n.utf8_text(source_code.as_bytes()).unwrap())
                    {
                        self.client
                            .log_message(MessageType::INFO, format!("4: {section_name}"))
                            .await;
                        return SectionType::from_str(section_name).ok();
                    }
                }
            }
        }

        None

        // // bubble up to find section
        // loop {
        //     match node.parent() {
        //         Some(parent) => {
        //             todo!();
        //
        //             node = parent;
        //         },
        //         None => break,
        //     }
        // }
        //
        // todo!()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> JsonRpcResult<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL, // TODO: incremental
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

        self.open_or_update(url, source_code).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.client
            .log_message(
                MessageType::INFO,
                format!("did_change: {}", params.text_document.uri),
            )
            .await;

        let url = params.text_document.uri;
        let source_code = params.content_changes.last().unwrap().text.to_string();

        for c in params.content_changes {
            // assume only changes
            if let Some(range) = c.range {
                self.client
                    .log_message(MessageType::INFO, format!("range: {:?}", range))
                    .await;
            } else {
                self.client
                    .log_message(MessageType::INFO, format!("full text change"))
                    .await;
            }
        }

        //
        // let old_tree = self.map.read().await
        //     .get(&url);
        // let new_tree = get_parser().await
        //     .parse(source_code, old_tree)
        //     .unwrap();
        //
        // self.map.write().await
        //     .insert(url, new_tree);
        //
        self.open_or_update(url, source_code.as_str()).await;
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

        // if let Some(tree) = self.map.read().await.get(&text_document.uri) {
        //     let root_node = tree.root_node();
        //     if let Some(node) = root_node.descendant_for_point_range(point, point) {
        //         // let a = match node.kind() {
        //         //     "abc" => "123",
        //         //     _ => "456",
        //         // };
        //
        //         // find current section
        //         // root_node.child_containing_descendant()
        //     }
        // }
        //
        // get_parser().await.parse()

        // TEMP
        let section_type = self
            .get_section_type_at_point(text_document.uri, &point)
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
                    ret.append(INPUT_COMPLETIONS.clone().as_mut());
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

        // temp
        // Ok(Some(CompletionResponse::Array(
        //     vec![
        //         CompletionItem {
        //             label: "label".to_string(),
        //             kind: Some(CompletionItemKind::FUNCTION),
        //             detail: Some("detail".to_string()),
        //             insert_text: Some("insert_text".to_string()),
        //             insert_text_format: Some(InsertTextFormat::SNIPPET),
        //
        //             ..CompletionItem::default()
        //         },
        //         CompletionItem::new_simple("label2".to_string(), "detail2".to_string()),
        //     ]
        // )))
    }
}
