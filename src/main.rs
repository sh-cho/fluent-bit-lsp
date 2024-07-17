use std::collections::HashMap;
use serde::Deserialize;

use tokio::sync::RwLock;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tower_lsp::jsonrpc::Result as JsonRpcResult;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializedParams, InitializeParams, InitializeResult, InsertTextFormat, MessageType, ServerCapabilities, TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, Url};
use tree_sitter::{Parser, Point, Tree};

struct MyParser {
    parser: Parser,  // Should I hold parser?
    tree: Tree,
    source_code: String,  // TODO: &str ?
}

impl MyParser {
    fn new(source_code: &str) -> Self {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fluentbit::language())
            .expect("Error loading fluentbit grammar");

        let tree = parser.parse(source_code, None)
            .unwrap();

        Self {
            parser,
            tree,
            source_code: source_code.to_string(),
        }
    }

    fn update(&mut self, source_code: &str) {
        let new_tree = self.parser.parse(source_code, Some(&self.tree))
            .unwrap();

        self.tree = new_tree;
        self.source_code = source_code.to_string();
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SectionType {
    Input,
    Parser,
    Filter,
    Output,

    #[serde(untagged)]
    Other(String),
}

struct Backend {
    client: Client,
    map: RwLock<HashMap<Url, MyParser>>,
}

impl Backend {
    async fn open_or_update(&self, url: Url, source_code: &str) {
        let mut wr = self.map.write().await;

        match wr.get_mut(&url) {
            Some(my_parser) => {
                my_parser.update(source_code);
            }
            None => {
                wr.insert(url, MyParser::new(source_code));
            }
        }
    }

    /// TODO: use TreeCursor
    async fn get_component_type_at_point(&self, url: Url, point: &Point) -> Option<SectionType> {
        let r = self.map.read().await;
        let Some(MyParser { parser, tree, source_code }) = r.get(&url) else {
            return None;
        };
        
        // let root_node = tree.root_node();

        let Some(mut node) = tree.root_node().descendant_for_point_range(*point, *point) else {
            return None;
        };

        self.client.log_message(
            MessageType::INFO, 
            format!("node.kind: {:?} / node: {:?} / point: {:?}", node.kind(), node.clone(), point)
        ).await;

        if node.kind() == "section" {
            let section_name = node.child_by_field_name("header")
                .and_then(|n| node.child_by_field_name("name"))
                .map(|n| n.utf8_text(source_code.as_bytes()).unwrap() )
                .unwrap();

            return serde_json::from_str::<SectionType>(section_name).ok();
        }

        // bubble up to find component
        loop {
            match node.parent() {
                Some(parent) => {


                    node = parent;
                },
                None => break,
            }
        }

        // todo!()
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
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                    completion_item: None,
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
            .log_message(MessageType::INFO, format!("file opened / {}", params.text_document.uri))
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

        // Only for full-text change + apply last? update
        let url = params.text_document.uri;
        let source_code = params.content_changes.last().unwrap()
            .text.to_string();
        //
        // let old_tree = self.map.read().await
        //     .get(&url);
        // let new_tree = get_parser().await
        //     .parse(source_code, old_tree)
        //     .unwrap();
        //
        // self.map.write().await
        //     .insert(url, new_tree);

        self.open_or_update(url, source_code.as_str()).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("did_close: {}", params.text_document.uri))
            .await;

        let url = params.text_document.uri;
        // self.map.borrow_mut()
        //     .remove(&url);

        self.map.write().await
            .remove(&url);
    }

    async fn completion(&self, params: CompletionParams) -> JsonRpcResult<Option<CompletionResponse>> {
        let TextDocumentPositionParams {
            text_document,
            position
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
        let section_type = self.get_component_type_at_point(text_document.uri, &point).await;
        let mut ret: Vec<CompletionItem> = Vec::new();
        
        self.client.log_message(MessageType::INFO, format!("section_type: {:?}", section_type)).await;
        
        if let Some(section) = section_type {
            match section {
                SectionType::Input => {
                    ret.push(CompletionItem::new_simple("InputLabel".to_string(), "InputDetail".to_string()));
                }
                SectionType::Parser => {
                    ret.push(CompletionItem::new_simple("ParserLabel".to_string(), "ParserDetail".to_string()));
                }
                SectionType::Filter => {
                    ret.push(CompletionItem::new_simple("FilterLabel".to_string(), "FilterDetail".to_string()));
                }
                SectionType::Output => {
                    ret.push(CompletionItem::new_simple("OutputLabel".to_string(), "OutputDetail".to_string()));
                }
                SectionType::Other(_) => {
                    ret.push(CompletionItem::new_simple("OtherLabel".to_string(), "OtherDetail".to_string()));
                }
            }
        } else {
            return Ok(None)
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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        map: RwLock::new(HashMap::new()),
    }).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parser() {
        let source_code = "
[INPUT]
    KEY1    VALUE1
";
        // let tree = get_parser().await
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fluentbit::language()).unwrap();

        let tree = parser.parse(source_code, None)
            .unwrap();

        let root_node = tree.root_node();

        // TODO: use NODE_TYPES?
        assert_eq!(root_node.kind(), "config");
    }
}
