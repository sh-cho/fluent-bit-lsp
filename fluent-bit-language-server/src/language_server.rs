use std::{collections::HashMap, str::FromStr};

use ropey::Rope;
use tokio::sync::RwLock;
use tower_lsp::{
    jsonrpc::Result as JsonRpcResult,
    lsp_types::{
        CompletionItem, CompletionOptions, CompletionOptionsCompletionItem, CompletionParams,
        CompletionResponse, Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities,
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
        FullDocumentDiagnosticReport, Hover, HoverContents, HoverParams, HoverProviderCapability,
        InitializeParams, InitializeResult, InitializedParams, MessageType, Position, Range,
        RelatedFullDocumentDiagnosticReport, ServerCapabilities, TextDocumentContentChangeEvent,
        TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    },
    Client, LanguageServer,
};
use tree_sitter::{Node, Point};

use crate::{
    completion::{get_completion, get_hover_info, FlbSectionType},
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

    pub async fn get_section_type_at_point(
        &self,
        url: &Url,
        point: &Point,
    ) -> Option<FlbSectionType> {
        let r = self.map.read().await;
        let TextDocument { rope, tree, .. } = r.get(url)?;
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
                    Self::get_section_name(&parent, rope)
                        .and_then(|name| FlbSectionType::from_str(&name).ok())
                } else {
                    None
                }
            }
            "key_type" => {
                // should go up parent tree until it finds section node
                let mut parent = node.parent();
                while let Some(p) = parent {
                    if let Some(section_name) = Self::get_section_name(&p, rope) {
                        return FlbSectionType::from_str(&section_name).ok();
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
                let section_name = rope.slice(byte_range).as_str()?;
                return Some(section_name.to_string());
            }
        }
        None
    }

    pub async fn get_key_at_point(&self, url: &Url, point: &Point) -> Option<String> {
        let r = self.map.read().await;
        let TextDocument { rope, tree, .. } = r.get(url)?;
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

    // There are some false-positive ERROR nodes in AST, due to reason below
    // (https://github.com/sh-cho/tree-sitter-fluentbit/pull/20)
    // So only simple check is done for now...
    //
    // ```fluentbit
    // [INPUT]  # ERROR COMMENT
    //     #    ^^^^^ Comment is not allowed here
    //     Name  tail
    //     #...
    // ```
    //
    pub async fn get_diagnostics(&self, url: &Url) -> Option<Vec<Diagnostic>> {
        let r = self.map.read().await;
        let TextDocument { tree, .. } = r.get(url)?;
        let Some(tree) = tree else { return None };

        let mut diagnostics = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();

        // So, Find "ERROR" node and check if it has "comment" node inside.
        // --
        // config: [0, 0] - [29, 0]
        //  section [7, 0] - [10, 0]
        //     header: section_header [7, 0] - [8, 0]
        //       name: section_header_type [7, 1] - [7, 17]
        //       ERROR [7, 18] - [7, 25]   # check this
        //         comment [7, 20] - [7, 25]
        // ...

        'outer: loop {
            if cursor.node().kind() == "ERROR" {
                let error_node = cursor.node();
                let mut error_cursor = error_node.walk();
                while error_cursor.goto_first_child() {
                    if error_cursor.node().kind() == "comment" {
                        let range = error_cursor.node().range();
                        let diagnostic = Diagnostic::new_simple(
                            Range::new(
                                Position::new(
                                    range.start_point.row as u32,
                                    range.start_point.column as u32,
                                ),
                                Position::new(
                                    range.end_point.row as u32,
                                    range.end_point.column as u32,
                                ),
                            ),
                            r"Comment is not allowed here.".to_string(),
                        );
                        diagnostics.push(diagnostic);
                    }
                }
            }

            // Traverse
            if cursor.goto_first_child() {
                continue 'outer;
            }
            if cursor.goto_next_sibling() {
                continue 'outer;
            }

            'inner: loop {
                if !cursor.goto_parent() {
                    break 'outer;
                }

                if cursor.goto_next_sibling() {
                    break 'inner;
                }
            }
        }

        Some(diagnostics)
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
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    // TODO: Real diagnostics
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: Default::default(),
                    },
                )),
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
                    .log_message(MessageType::INFO, "full text change".to_string())
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

    // TODO: Supply snippet only when there's no "Name" entry
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

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> JsonRpcResult<DocumentDiagnosticReportResult> {
        // for test just mark second line is error

        let DocumentDiagnosticParams { text_document, .. } = params;
        let url = text_document.uri;

        // let diagnostics: Vec<Diagnostic> = vec![Diagnostic::new_simple(
        //     Range::new(Position::new(1, 0), Position::new(2, 10)),
        //     "error message".to_string(),
        // )];

        let diagnostics = self.get_diagnostics(&url).await.unwrap_or_default();

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    items: diagnostics,
                    result_id: None,
                },
                related_documents: None,
            }),
        ))
    }
}
