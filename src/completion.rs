use std::string::ToString;

use once_cell::sync::Lazy;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind,
};

pub static INPUT_COMPLETIONS: Lazy<Vec<CompletionItem>> = Lazy::new(|| {
    let mut ret = vec![];

    ret.push(CompletionItem {
        kind: Some(CompletionItemKind::SNIPPET),
        label: "Kafka".to_string(),
        label_details: Some(CompletionItemLabelDetails {
            detail: Some("label_detail".to_string()),
            description: Some("label_detail_desc".to_string()),
        }),
        detail: Some("detail_str".to_string()),
        deprecated: Some(true),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: r####"# Header
Some text
```rust
use std::string::ToString;

let a = 1;
```"####
                .to_string(),
        })),
        //         documentation: Some(Documentation::String("doc_str".to_string())),
        insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(
            r#"brokers             ${1:kafka:9092}
topics              ${2:my_topic}
poll_ms             ${3:500}
Buffer_Max_Size     ${4:4M}
$0"#
            .to_string(),
        ),
        // text_edit: Some(CompletionT),
        ..CompletionItem::default()
    });

    ret
});
