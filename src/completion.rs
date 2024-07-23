use std::string::ToString;
use once_cell::sync::Lazy;

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind,
};

#[derive(Clone)]
pub(crate) struct FlbCompletionSnippet {
    label: String,
    detail: Option<String>,
    documentation_markdown: String,
    label_details: Option<String>,
    label_details_desc: Option<String>,
    insert_text: String,
}

impl From<FlbCompletionSnippet> for CompletionItem {
    fn from(snippet: FlbCompletionSnippet) -> Self {
        CompletionItem {
            kind: Some(CompletionItemKind::SNIPPET),
            label: snippet.label,
            label_details: Some(CompletionItemLabelDetails {
                detail: snippet.label_details,
                description: snippet.label_details_desc,
            }),
            detail: snippet.detail,
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: snippet.documentation_markdown,
            })),
            insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            insert_text: Some(snippet.insert_text),
            ..CompletionItem::default()
        }
    }
}

pub static INPUT_COMPLETIONS: Lazy<Vec<FlbCompletionSnippet>> = Lazy::new(|| {
    let mut ret = vec![];

    ret.push(FlbCompletionSnippet {
        label: "Kafka".to_string(),
        label_details: Some("label_detail".to_string()),
        detail: Some("detail_str".to_string()),
        documentation_markdown: r####"# Header
Some text
```rust
use std::string::ToString;

let a = 1;
```"####
            .to_string(),
        label_details_desc: Some("label_detail_desc".to_string()),
        insert_text: r#"brokers             ${1:kafka:9092}
topics              ${2:my_topic}
poll_ms             ${3:500}
Buffer_Max_Size     ${4:4M}
$0"#
            .to_string(),
    });

    ret
});
