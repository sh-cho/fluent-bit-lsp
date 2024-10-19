use std::{collections::HashMap, string::ToString};

use convert_case::{Case, Casing};
use flb_schema::section::FlbSectionType;
/// TODO: sort out generated code
#[allow(unused_imports)]
use once_cell::sync::Lazy;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation, Hover,
    HoverContents, InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlbConfigParameterInfo {
    pub(crate) default_value: Option<String>,
    pub(crate) description: String,
}

impl FlbConfigParameterInfo {
    pub fn to_hover(&self, markup_kind: MarkupKind) -> Hover {
        let mut value = self.description.clone();
        if let Some(default_value) = &self.default_value {
            value.push_str(format!("\n\n(Default: `{}`)", default_value).as_str());
        }

        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: markup_kind,
                value,
            }),
            range: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlbConfigParameter {
    pub(crate) key: String,
    pub(crate) info: FlbConfigParameterInfo,
}

impl FlbConfigParameter {
    fn new(key: &str, default_value: Option<&str>, description: &str) -> Self {
        Self {
            key: key.to_string(),
            info: FlbConfigParameterInfo {
                default_value: default_value.map(|s| s.to_string()),
                description: description.to_string(),
            },
        }
    }

    fn to_insert_text(&self, tab_stop: usize, key_width: usize) -> String {
        assert!(tab_stop > 0);

        let value_str = match &self.info.default_value {
            Some(val) => format!("${{{tab_stop}:{}}}", val),
            None => format!("${tab_stop}"),
        };

        format!("{:key_width$} {}", self.key, value_str)
    }
}

#[derive(Clone)]
pub(crate) struct FlbCompletionSnippet {
    /// Completion Label which will be printed in the completion list
    ///
    /// e.g. "Network I/O Metrics"
    label: String,

    /// Plugin name which will be used in the configuration file
    ///
    /// e.g. `netif`
    plugin_name: String,
    documentation_markdown: String,
    config_params: Vec<FlbConfigParameter>,
    // XXX: maybe no need
    // detail: Option<String>,
    // label_details: Option<String>,
    // label_details_desc: Option<String>,
}

impl FlbCompletionSnippet {
    pub fn new(
        label: &str,
        plugin_name: Option<&str>,
        documentation_markdown: &str,
        config_params: Vec<FlbConfigParameter>,
    ) -> Self {
        FlbCompletionSnippet {
            label: label.to_string(),
            plugin_name: plugin_name.map_or_else(|| label.to_case(Case::Snake), |s| s.to_string()),
            documentation_markdown: documentation_markdown.to_string(),
            config_params,
        }
    }

    pub fn props_to_insert_text(&self) -> String {
        const KEY_WIDTH: usize = 15; // TODO: dynamic?

        let mut ret = format!("{:KEY_WIDTH$} {}\n", "Name", self.plugin_name);

        for (index, param) in self.config_params.iter().enumerate() {
            let tab_stop = index + 1;
            let line = param.to_insert_text(tab_stop, KEY_WIDTH);
            ret.push_str(format!("{}\n", line).as_str());
        }

        ret
    }

    // TODO: cache
    pub fn documentation_plaintext(&self) -> String {
        let mut ret = format!("{}: {}\n\n", self.plugin_name, self.label);

        ret.push_str("[parameters]\n");
        for param in &self.config_params {
            ret.push_str(format!("- {}: {}\n", param.key, param.info.description).as_str());
        }

        ret
    }
}

pub fn snippet_to_completion(
    snippet: FlbCompletionSnippet,
    section_type: &FlbSectionType,
    markup_kind: MarkupKind,
) -> CompletionItem {
    let insert_text = snippet.props_to_insert_text();
    let documentation_string = match markup_kind {
        MarkupKind::PlainText => snippet.documentation_plaintext(),
        MarkupKind::Markdown => snippet.documentation_markdown,
    };

    CompletionItem {
        kind: Some(CompletionItemKind::SNIPPET),
        label: snippet.label,
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: Some(format!("{} plugin", section_type)),
        }),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: markup_kind,
            value: documentation_string,
        })),
        insert_text_mode: Some(InsertTextMode::ADJUST_INDENTATION),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        insert_text: Some(insert_text),
        ..CompletionItem::default()
    }
}

// static datas for completion, hover, etc
pub struct FlbData {
    pub(crate) snippets: HashMap<FlbSectionType, Vec<FlbCompletionSnippet>>,
    pub(crate) params: HashMap<(FlbSectionType, String), FlbConfigParameterInfo>,
}

impl FlbData {
    pub fn new() -> Self {
        FlbData {
            snippets: HashMap::new(),
            params: HashMap::new(),
        }
    }

    pub fn add_snippet(&mut self, section_type: FlbSectionType, snippet: FlbCompletionSnippet) {
        self.snippets
            .entry(section_type.clone())
            .or_default()
            .push(snippet.clone());

        // insert params
        snippet.config_params.iter().for_each(|param| {
            self.params.insert(
                (section_type.clone(), param.key.clone().to_lowercase()),
                param.info.clone(),
            );
        });
    }

    pub fn get_snippets(
        &self,
        section_type: &FlbSectionType,
    ) -> Option<&Vec<FlbCompletionSnippet>> {
        self.snippets.get(section_type)
    }

    pub fn get_parameter_info(
        &self,
        section_type: &FlbSectionType,
        key: &str,
    ) -> Option<&FlbConfigParameterInfo> {
        self.params.get(&(section_type.clone(), key.to_string()))
    }
}

macro_rules! read_flb_docs {
    ($section:literal, $name:literal) => {
        include_str!(concat!("assets/docs/", $section, "/", $name, ".md"))
    };

    ($path:literal) => {
        include_str!(concat!("assets/docs/", $path, ".md"))
    };
}

macro_rules! add_snippet {
    (
        $flb_data:expr,
        $section:expr,
        $label:expr,
        $doc_path:expr,
        [
            $(
                ($key:expr, $default:expr, $desc:expr)
            ),*
            $(,)?
        ]
    ) => {
        let config_params = vec![
            $(
                FlbConfigParameter::new($key, $default, $desc),
            )*
        ];
        let snippet = FlbCompletionSnippet::new($label, None, read_flb_docs!($doc_path), config_params);
        $flb_data.add_snippet($section, snippet);
    };

    (
        $flb_data:expr,
        $section:expr,
        $label:expr,
        $plugin_name:expr,
        $doc_path:expr,
        [
            $(
                ($key:expr, $default:expr, $desc:expr)
            ),*
            $(,)?
        ]
    ) => {
        let config_params = vec![
            $(
                FlbConfigParameter::new($key, $default, $desc),
            )*
        ];
        let snippet = FlbCompletionSnippet::new($label, Some($plugin_name), read_flb_docs!($doc_path), config_params);
        $flb_data.add_snippet($section, snippet);
    };
}

include!("schema.generated.rs");

pub fn get_completion(
    section_type: &FlbSectionType,
    markup_kind: MarkupKind,
) -> Vec<CompletionItem> {
    FLB_DATA
        .get_snippets(section_type)
        .unwrap_or(&vec![])
        .iter()
        .map(|snippet| snippet_to_completion(snippet.clone(), section_type, markup_kind.clone()))
        .collect()
}

pub fn get_hover_info(section_type: &FlbSectionType, key: &str) -> Option<FlbConfigParameterInfo> {
    FLB_DATA
        .get_parameter_info(section_type, key.to_lowercase().as_str())
        .cloned()
}
