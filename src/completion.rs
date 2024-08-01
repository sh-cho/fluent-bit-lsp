use std::{collections::HashMap, str::FromStr, string::ToString};

use once_cell::sync::Lazy;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum SectionType {
    Input,
    Parser,
    MultilineParser,
    Filter,
    Output,

    Other(String),
}

impl FromStr for SectionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "INPUT" => SectionType::Input,
            "PARSER" => SectionType::Parser,
            "MULTILINE_PARSER" => SectionType::MultilineParser,
            "FILTER" => SectionType::Filter,
            "OUTPUT" => SectionType::Output,
            _ => SectionType::Other(s.to_string()),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FlbConfigParameterInfo {
    pub(crate) default_value: Option<String>,
    pub(crate) description: String,
}

impl From<FlbConfigParameterInfo> for MarkupContent {
    fn from(info: FlbConfigParameterInfo) -> Self {
        let mut value = info.description.clone();
        if let Some(default_value) = info.default_value {
            value.push_str(format!("\n\n(Default: `{}`)", default_value).as_str());
        }

        MarkupContent {
            kind: MarkupKind::Markdown,
            value,
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
    label: String,
    detail: Option<String>,
    documentation_markdown: String, // TODO:
    label_details: Option<String>,
    label_details_desc: Option<String>,
    config_params: Vec<FlbConfigParameter>,
}

impl FlbCompletionSnippet {
    pub fn props_to_insert_text(&self) -> String {
        const KEY_WIDTH: usize = 15; // TODO: dynamic?

        let mut ret = format!("{:KEY_WIDTH$} {}\n", "Name", self.label.to_lowercase());

        for (index, param) in self.config_params.iter().enumerate() {
            let tab_stop = index + 1;
            let line = param.to_insert_text(tab_stop, KEY_WIDTH);
            ret.push_str(format!("{}\n", line).as_str());
        }

        ret
    }
}

impl From<FlbCompletionSnippet> for CompletionItem {
    fn from(snippet: FlbCompletionSnippet) -> Self {
        let insert_text = snippet.props_to_insert_text();

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
            insert_text: Some(insert_text),
            ..CompletionItem::default()
        }
    }
}

pub struct FlbData {
    pub(crate) snippets: HashMap<SectionType, Vec<FlbCompletionSnippet>>,
    pub(crate) params: HashMap<(SectionType, String), FlbConfigParameterInfo>,
}

impl FlbData {
    pub fn new() -> Self {
        FlbData {
            snippets: HashMap::new(),
            params: HashMap::new(),
        }
    }

    pub fn add_snippet(&mut self, section_type: SectionType, snippet: FlbCompletionSnippet) {
        self.snippets
            .entry(section_type.clone())
            .or_insert_with(Vec::new)
            .push(snippet.clone());

        // insert params
        snippet.config_params.iter().for_each(|param| {
            self.params.insert(
                (section_type.clone(), param.key.clone().to_lowercase()),
                param.info.clone(),
            );
        });
    }

    pub fn get_snippets(&self, section_type: &SectionType) -> Option<&Vec<FlbCompletionSnippet>> {
        self.snippets.get(section_type)
    }

    pub fn get_parameter_info(
        &self,
        section_type: &SectionType,
        key: &str,
    ) -> Option<&FlbConfigParameterInfo> {
        self.params.get(&(section_type.clone(), key.to_string()))
    }
}

macro_rules! read_flb_docs {
    ($section:literal, $name:literal) => {
        include_str!(concat!("assets/docs/", $section, "/", $name, ".md"))
    };
}

pub static FLB_DATA: Lazy<FlbData> = Lazy::new(|| {
    let mut data = FlbData::new();

    data.add_snippet(
        SectionType::Input,
        FlbCompletionSnippet {
            label: "Kafka".to_string(),
            label_details: Some("label_detail".to_string()),
            detail: Some("detail_str".to_string()),
            documentation_markdown: read_flb_docs!("input", "kafka").to_string(),
            label_details_desc: Some("label_detail_desc".to_string()),
            config_params: vec![
                FlbConfigParameter::new("brokers", Some("kafka:9092"), "Single or multiple list of Kafka Brokers, e.g: 192.168.1.3:9092, 192.168.1.4:9092."),
                FlbConfigParameter::new("topics", Some("my_topic"), "Single entry or list of topics separated by comma (,) that Fluent Bit will subscribe to."),
                FlbConfigParameter::new("format", Some("none"), r#"Serialization format of the messages. If set to "json", the payload will be parsed as json."#),
                FlbConfigParameter::new("client_id", None, "Client id passed to librdkafka."),
                FlbConfigParameter::new("group_id", None, "Client id passed to librdkafka."),
                FlbConfigParameter::new("poll_ms", Some("500"), "Kafka brokers polling interval in milliseconds."),
                FlbConfigParameter::new("Buffer_Max_Size", Some("4M"), "Specify the maximum size of buffer per cycle to poll kafka messages from subscribed topics. To increase throughput, specify larger size."),
            ],
        },
    );
    data.add_snippet(
        SectionType::Input,
        FlbCompletionSnippet {
            label: "Collectd".to_string(),
            label_details: Some("label_detail".to_string()),
            detail: Some("detail_str".to_string()),
            documentation_markdown: read_flb_docs!("input", "collectd").to_string(),
            label_details_desc: Some("label_detail_desc".to_string()),
            config_params: vec![
                FlbConfigParameter::new("Listen", Some("0.0.0.0"), "Set the address to listen to"),
                FlbConfigParameter::new("Port", Some("25826"), "Set the port to listen to"),
                FlbConfigParameter::new(
                    "TypesDB",
                    Some("/usr/share/collectd/types.db"),
                    "Set the data specification file",
                ),
            ],
        },
    );

    data.add_snippet(
        SectionType::Output,
        FlbCompletionSnippet {
            label: "Kafka".to_string(),
            label_details: Some("label_detail".to_string()),
            detail: Some("detail_str".to_string()),
            documentation_markdown: read_flb_docs!("output", "kafka").to_string(),
            label_details_desc: Some("label_detail_desc".to_string()),
            config_params: vec![
                FlbConfigParameter::new("format", Some("json"), "Specify data format, options available: json, msgpack, raw."),
                FlbConfigParameter::new("message_key", None, "Optional key to store the message"),
                FlbConfigParameter::new("message_key_field", None, "If set, the value of Message_Key_Field in the record will indicate the message key. If not set nor found in the record, Message_Key will be used (if set)."),
                FlbConfigParameter::new("timestamp_key", Some("@timestamp"), "Set the key to store the record timestamp"),
                FlbConfigParameter::new("timestamp_format", Some("double"), "Specify timestamp format, should be 'double', 'iso8601' (seconds precision) or 'iso8601_ns' (fractional seconds precision)"),
                FlbConfigParameter::new("brokers", None, "Single or multiple list of Kafka Brokers, e.g: 192.168.1.3:9092, 192.168.1.4:9092."),
                FlbConfigParameter::new("topics", Some("fluent-bit"), "Single entry or list of topics separated by comma (,) that Fluent Bit will use to send messages to Kafka. If only one topic is set, that one will be used for all records. Instead if multiple topics exists, the one set in the record by Topic_Key will be used."),
                FlbConfigParameter::new("topic_key", None, r#"If multiple Topics exists, the value of Topic_Key in the record will indicate the topic to use. E.g: if Topic_Key is router and the record is {"key1": 123, "router": "route_2"}, Fluent Bit will use topic route_2. Note that if the value of Topic_Key is not present in Topics, then by default the first topic in the Topics list will indicate the topic to be used."#),
                FlbConfigParameter::new("dynamic_topic", Some("Off"), "adds unknown topics (found in Topic_Key) to Topics. So in Topics only a default topic needs to be configured."),
                FlbConfigParameter::new("queue_full_retries", Some("10"), "Fluent Bit queues data into rdkafka library, if for some reason the underlying library cannot flush the records the queue might fills up blocking new addition of records. The queue_full_retries option set the number of local retries to enqueue the data. The default value is 10 times, the interval between each retry is 1 second. Setting the queue_full_retries value to 0 set's an unlimited number of retries."),
                FlbConfigParameter::new("raw_log_key", None, "When using the raw format and set, the value of raw_log_key in the record will be send to kafka as the payload."),
                FlbConfigParameter::new("workers", Some("0"), "This setting improves the throughput and performance of data forwarding by enabling concurrent data processing and transmission to the kafka output broker destination."),
                // FlbConfigParameter::new("rdkafka.{property}", None, "{property} can be any librdkafka properties"),
            ],
        }
    );
    data.add_snippet(
        SectionType::Output,
        FlbCompletionSnippet {
            label: "File".to_string(),
            label_details: Some("label_detail".to_string()),
            detail: Some("detail_str".to_string()),
            documentation_markdown: read_flb_docs!("output", "file").to_string(),
            label_details_desc: Some("label_detail_desc".to_string()),
            config_params: vec![
                FlbConfigParameter::new("Path", None, "Directory path to store files. If not set, Fluent Bit will write the files on it's own positioned directory. note: this option was added on Fluent Bit v1.4.6"),
                FlbConfigParameter::new("File", None, "Set file name to store the records. If not set, the file name will be the tag associated with the records."),
                FlbConfigParameter::new("Format", None, "The format of the file content. See also Format section. Default: out_file."),
                FlbConfigParameter::new("Mkdir", None, "Recursively create output directory if it does not exist. Permissions set to 0755."),
                FlbConfigParameter::new("Workers", Some("1"), "Enables dedicated thread(s) for this output. Default value is set since version 1.8.13. For previous versions is 0."),
            ],
        }
    );

    data
});

pub fn get_completion(section_type: &SectionType) -> Vec<CompletionItem> {
    FLB_DATA
        .get_snippets(section_type)
        .unwrap_or(&vec![])
        .iter()
        .map(|snippet| snippet.clone().into())
        .collect()
}

pub fn get_hover_info(section_type: &SectionType, key: &str) -> Option<FlbConfigParameterInfo> {
    FLB_DATA
        .get_parameter_info(section_type, key.to_lowercase().as_str())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_name_to_enum() {
        let section: SectionType = str::parse("input").unwrap();
        assert_eq!(section, SectionType::Input);
    }
}
