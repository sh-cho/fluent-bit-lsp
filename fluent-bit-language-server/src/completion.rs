use std::{collections::HashMap, fmt::Display, str::FromStr, string::ToString};

use once_cell::sync::Lazy;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, Documentation,
    InsertTextFormat, InsertTextMode, MarkupContent, MarkupKind,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum FlbSectionType {
    Input,
    Parser,
    MultilineParser,
    Filter,
    Output,

    Other(String),
}

impl FromStr for FlbSectionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "INPUT" => FlbSectionType::Input,
            "PARSER" => FlbSectionType::Parser,
            "MULTILINE_PARSER" => FlbSectionType::MultilineParser,
            "FILTER" => FlbSectionType::Filter,
            "OUTPUT" => FlbSectionType::Output,
            _ => FlbSectionType::Other(s.to_string()),
        })
    }
}

impl Display for FlbSectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            FlbSectionType::Input => "input".to_string(),
            FlbSectionType::Parser => "parser".to_string(),
            FlbSectionType::MultilineParser => "multiline_parser".to_string(),
            FlbSectionType::Filter => "filter".to_string(),
            FlbSectionType::Output => "output".to_string(),
            FlbSectionType::Other(s) => s.clone(),
        };
        write!(f, "{}", str)
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
    documentation_markdown: String,
    config_params: Vec<FlbConfigParameter>,
    // detail: Option<String>,
    // label_details: Option<String>,
    // label_details_desc: Option<String>,
}

impl FlbCompletionSnippet {
    pub fn new(
        label: &str,
        documentation_markdown: &str,
        config_params: Vec<FlbConfigParameter>,
    ) -> Self {
        FlbCompletionSnippet {
            label: label.to_string(),
            documentation_markdown: documentation_markdown.to_string(),
            config_params,
        }
    }

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

pub fn snippet_to_completion(
    snippet: FlbCompletionSnippet,
    section_type: &FlbSectionType,
) -> CompletionItem {
    let insert_text = snippet.props_to_insert_text();

    CompletionItem {
        kind: Some(CompletionItemKind::SNIPPET),
        label: snippet.label,
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: Some(format!("{} plugin", section_type)),
        }),
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
        let snippet = FlbCompletionSnippet::new($label, read_flb_docs!($doc_path), config_params);
        $flb_data.add_snippet($section, snippet);
    };
}

// XXX: better way than copy-paste manually?
#[rustfmt::skip::macros(add_snippet)]
static FLB_DATA: Lazy<FlbData> = Lazy::new(|| {
    let mut data = FlbData::new();

    //////////////////////////////////////////////////////////////////////////////////////////
    // Input
    add_snippet!(data, FlbSectionType::Input, "Kafka", "input/kafka", [
        ("brokers", Some("kafka:9092"), "Single or multiple list of Kafka Brokers, e.g: 192.168.1.3:9092, 192.168.1.4:9092."),
        ("topics", Some("my_topic"), "Single entry or list of topics separated by comma (,) that Fluent Bit will subscribe to."),
        ("format", Some("none"), r#"Serialization format of the messages. If set to "json", the payload will be parsed as json."#),
        ("client_id", None, "Client id passed to librdkafka."),
        ("group_id", None, "Group id passed to librdkafka."),
        ("poll_ms", Some("500"), "Kafka brokers polling interval in milliseconds."),
        ("Buffer_Max_Size", Some("4M"), "Specify the maximum size of buffer per cycle to poll kafka messages from subscribed topics. To increase throughput, specify larger size."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Collectd", "input/collectd", [
        ("Listen", Some("0.0.0.0"), "Set the address to listen to"),
        ("Port", Some("25826"), "Set the port to listen to"),
        ("TypesDB", Some("/usr/share/collectd/types.db"), "Set the data specification file"),
    ]);
    add_snippet!(data, FlbSectionType::Input, "CPU", "input/cpu-metrics", [
        ("Interval_Sec", Some("1"), "Polling interval in seconds"),
        ("Interval_NSec", Some("0"), "Polling interval in nanoseconds"),
        ("PID", None, "Specify the ID (PID) of a running process in the system. By default the plugin monitors the whole system but if this option is set, it will only monitor the given process ID."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Disk", "input/disk-io-metrics", [
        ("Interval_Sec", Some("1"), "Polling interval in seconds"),
        ("Interval_NSec", Some("0"), "Polling interval in nanoseconds"),
        ("Dev_Name", None, "Device name to limit the target. (e.g. sda). If not set, in_disk gathers information from all of disks and partitions."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Docker Metrics", "input/docker-metrics", [
        ("Interval_Sec", Some("1"), "Polling interval in seconds"),
        ("Include", None, "A space-separated list of containers to include"),
        ("Exclude", None, "A space-separated list of containers to exclude"),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Docker Events", "input/docker-events", [
        ("Unix_Path", Some("/var/run/docker.sock"), "The docker socket unix path"),
        ("Buffer_Size", Some("8192"), "The size of the buffer used to read docker events (in bytes)"),
        ("Parser", None, "Specify the name of a parser to interpret the entry as a structured message."),
        ("Key", Some("message"), "When a message is unstructured (no parser applied), it's appended as a string under the key name message."),
        ("Reconnect.Retry_limits", Some("5"), "The maximum number of retries allowed. The plugin tries to reconnect with docker socket when EOF is detected."),
        ("Reconnect.Retry_interval", Some("1"), "The retrying interval. Unit is second."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Dummy", "input/dummy", [
        ("Dummy", Some(r#"{"message":"dummy"}"#), "Dummy JSON record"),
        ("Metadata", Some("{}"), "Dummy JSON metadata"),
        ("Start_time_sec", Some("0"), "Dummy base timestamp in seconds"),
        ("Start_time_nsec", Some("0"), "Dummy base timestamp in nanoseconds"),
        ("Rate", Some("1"), "Rate at which messages are generated expressed in how many times per second"),
        ("Interval_Sec", Some("0"), "Set seconds of time interval at which every message is generated. If set, `Rate` configuration will be ignored"),
        ("Interval_nsec", Some("0"), "Set nanoseconds of time interval at which every message is generated. If set, `Rate` configuration will be ignored"),
        ("Samples", None, "If set, the events number will be limited. e.g. If Samples=3, the plugin only generates three events and stops."),
        ("Copies", Some("1"), "Number of messages to generate each time they are generated"),
        ("Flush_on_startup", Some("false"), "If set to `true`, the first dummy event is generated at startup"),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Elasticsearch", "input/elasticsearch", [
        ("buffer_max_size", Some("4M"), "Set the maximum size of buffer."),
        ("buffer_chunk_size", Some("512K"), "Set the buffer chunk size."),
        ("tag_key", Some("NULL"), "Specify a key name for extracting as a tag."),
        ("meta_key", Some("@meta"), "Specify a key name for meta information."),
        ("hostname", Some("localhost"), r#"Specify hostname or FQDN. This parameter can be used for "sniffing" (auto-discovery of) cluster node information."#),
        ("version", Some("8.0.0"), "Specify Elasticsearch server version. This parameter is effective for checking a version of Elasticsearch/OpenSearch server version."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Exec", "input/exec", [
        ("Command", None, r#"The command to execute, passed to [popen(...)](https://man7.org/linux/man-pages/man3/popen.3.html) without any additional escaping or processing. May include pipelines, redirection, command-substitution, etc."#),
        ("Parser", None, "Specify the name of a parser to interpret the entry as a structured message."),
        ("Interval_Sec", None, "Polling interval (seconds)."),
        ("Interval_NSec", None, "Polling interval (nanosecond)."),
        ("Buf_Size", None, "Size of the buffer (check [unit sizes](https://docs.fluentbit.io/manual/administration/configuring-fluent-bit/unit-sizes) for allowed values)"),
        ("Oneshot", Some("false"), "Only run once at startup. This allows collection of data precedent to fluent-bit's startup"),
        ("Exit_After_Oneshot", Some("false"), "Exit as soon as the one-shot command exits. This allows the exec plugin to be used as a wrapper for another command, sending the target command's output to any fluent-bit sink(s) then exiting."),
        ("Propagate_Exit_Code", Some("false"), "When exiting due to Exit_After_Oneshot, cause fluent-bit to exit with the exit code of the command exited by this plugin. Follows [shell conventions for exit code propagation](https://www.gnu.org/software/bash/manual/html_node/Exit-Status.html)."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Exec Wasi", "input/exec-wasi", [
        ("WASI_Path", None, "The place of a WASM program file."),
        ("Parser", None, "Specify the name of a parser to interpret the entry as a structured message."),
        ("Accessible_Paths", None, "Specify the whilelist of paths to be able to access paths from WASM programs."),
        ("Interval_Sec", None, "Polling interval (seconds)."),
        ("Interval_NSec", None, "Polling interval (nanoseconds)."),
        ("Buf_Size", None, "Size of the buffer (check [unit sizes](https://docs.fluentbit.io/manual/administration/configuring-fluent-bit/unit-sizes) for allowed values)"),
        ("Oneshot", Some("false"), "Only run once at startup. This allows collection of data precedent to fluent-bit's startup"),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Fluent Bit Metrics", "input/fluentbit-metrics", [
        ("scrape_interval", Some("2"), "The rate at which metrics are collected from the host operating system"),
        ("scrape_on_start", Some("false"), "Scrape metrics upon start, useful to avoid waiting for `scrape_interval` for the first round of metrics."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Forward", "input/forward", [
        ("Listen", Some("0.0.0.0"), "Listener network interface"),
        ("Port", Some("24224"), "TCP port to listen for incoming connections"),
        ("Unix_Path", None, "Specify the path to unix socket to receive a Forward message. If set, `Listen` and `Port` are ignored."),
        ("Unix_Perm", None, "Set the permission of the unix socket file. If `Unix_Path` is not set, this parameter is ignored."),
        ("Buffer_Max_Size", Some("6144000"), "Specify the maximum buffer memory size used to receive a Forward message. The value must be according to the [Unit Size](https://docs.fluentbit.io/manual/administration/configuring-fluent-bit/unit-sizes) specification."),
        ("Buffer_Chunk_Size", Some("1024000"), "By default the buffer to store the incoming Forward messages, do not allocate the maximum memory allowed, instead it allocate memory when is required. The rounds of allocations are set by `Buffer_Chunk_Size`. The value must be according to the Unit Size specification."),
        ("Tag_Prefix", None, "Prefix incoming tag with the defined value."),
        ("Tag", None, "Override the tag of the forwarded events with the defined value."),
        ("Shared_Key", None, "Shared key for secure forward authentication."),
        ("Self_Hostname", None, "Hostname for secure forward authentication."),
        ("Security.Users", None, "Specify the username and password pairs for secure forward authentication."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Head", "input/head", [
        ("File", None, "Absolute path to the target file, e.g: /proc/uptime"),
        ("Buf_Size", None, "Buffer size to read the file."),
        ("Interval_Sec", None, "Polling interval (seconds)."),
        ("Interval_NSec", None, "Polling interval (nanoseconds)."),
        ("Add_Path", Some("false"), "If enabled, filepath is appended to each records."),
        ("Key", Some("head"), "Rename a key."),
        ("Lines", None, "Line number to read. If the number N is set, in_head reads first N lines like head(1) -n."),
        ("Split_line", None, "If enabled, in_head generates key-value pair per line."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "Health", "input/health", [
        ("Host", None, "Name of the target host or IP address to check."),
        ("Port", None, "TCP port where to perform the connection check."),
        ("Interval_Sec", Some("1"), "Interval in seconds between the service checks."),
        ("Interval_NSec", Some("0"), "Specify a nanoseconds interval for service checks, it works in conjunction with the Interval_Sec configuration key."),
        ("Alert", None, "If enabled, it will only generate messages if the target TCP service is down. By default this option is disabled."),
        ("Add_Host", Some("false"), "If enabled, hostname is appended to each records."),
        ("Add_Port", Some("false"), "If enabled, port number is appended to each records."),
    ]);
    add_snippet!(data, FlbSectionType::Input, "HTTP", "input/http", [
        ("listen", Some("0.0.0.0"), "The address to listen on"),
        ("port", Some("9880"), "The port for Fluent Bit to listen on"),
        ("tag_key", None, "Specify the key name to overwrite a tag. If set, the tag will be overwritten by a value of the key."),
        ("buffer_max_size", Some("4M"), "Specify the maximum buffer size in KB to receive a JSON message."),
        ("buffer_chunk_size", Some("512K"), "This sets the chunk size for incoming incoming JSON messages. These chunks are then stored/managed in the space available by `buffer_max_size`."),
        ("successful_response_code", Some("201"), "It allows to set successful response code. `200`, `201` and `204` are supported."),
        ("success_header", None, "Add an HTTP header key/value pair on success. Multiple headers can be set. Example: `X-Custom custom-answer`"),
    ]);

    //////////////////////////////////////////////////////////////////////////////////////////
    // Output
    add_snippet!(data, FlbSectionType::Output, "Kafka", "output/kafka", [
        ("format", Some("json"), "Specify data format, options available: json, msgpack, raw."),
        ("message_key", None, "Optional key to store the message"),
        ("message_key_field", None, "If set, the value of Message_Key_Field in the record will indicate the message key. If not set nor found in the record, Message_Key will be used (if set)."),
        ("timestamp_key", Some("@timestamp"), "Set the key to store the record timestamp"),
        ("timestamp_format", Some("double"), "Specify timestamp format, should be 'double', 'iso8601' (seconds precision) or 'iso8601_ns' (fractional seconds precision)"),
        ("brokers", None, "Single or multiple list of Kafka Brokers, e.g: 192.168.1.3:9092, 192.168.1.4:9092."),
        ("topics", Some("fluent-bit"), "Single entry or list of topics separated by comma (,) that Fluent Bit will use to send messages to Kafka. If only one topic is set, that one will be used for all records. Instead if multiple topics exists, the one set in the record by Topic_Key will be used."),
        ("topic_key", None, r#"If multiple Topics exists, the value of Topic_Key in the record will indicate the topic to use. E.g: if Topic_Key is router and the record is {"key1": 123, "router": "route_2"}, Fluent Bit will use topic route_2. Note that if the value of Topic_Key is not present in Topics, then by default the first topic in the Topics list will indicate the topic to be used."#),
        ("dynamic_topic", Some("Off"), "adds unknown topics (found in Topic_Key) to Topics. So in Topics only a default topic needs to be configured."),
        ("queue_full_retries", Some("10"), "Fluent Bit queues data into rdkafka library, if for some reason the underlying library cannot flush the records the queue might fills up blocking new addition of records. The queue_full_retries option set the number of local retries to enqueue the data. The default value is 10 times, the interval between each retry is 1 second. Setting the queue_full_retries value to 0 set's an unlimited number of retries."),
        ("raw_log_key", None, "When using the raw format and set, the value of raw_log_key in the record will be send to kafka as the payload."),
        ("workers", Some("0"), "This setting improves the throughput and performance of data forwarding by enabling concurrent data processing and transmission to the kafka output broker destination."),
    ]);
    add_snippet!(data, FlbSectionType::Output, "File", "output/file", [
        ("Path", None, "Directory path to store files. If not set, Fluent Bit will write the files on it's own positioned directory. note: this option was added on Fluent Bit v1.4.6"),
        ("File", None, "Set file name to store the records. If not set, the file name will be the tag associated with the records."),
        ("Format", None, "The format of the file content. See also Format section. Default: out_file."),
        ("Mkdir", None, "Recursively create output directory if it does not exist. Permissions set to 0755."),
        ("Workers", Some("1"), "Enables dedicated thread(s) for this output. Default value is set since version 1.8.13. For previous versions is 0."),
    ]);

    data
});

pub fn get_completion(section_type: &FlbSectionType) -> Vec<CompletionItem> {
    FLB_DATA
        .get_snippets(section_type)
        .unwrap_or(&vec![])
        .iter()
        .map(|snippet| snippet_to_completion(snippet.clone(), section_type))
        .collect()
}

pub fn get_hover_info(section_type: &FlbSectionType, key: &str) -> Option<FlbConfigParameterInfo> {
    FLB_DATA
        .get_parameter_info(section_type, key.to_lowercase().as_str())
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_name_to_enum() {
        let section: FlbSectionType = str::parse("input").unwrap();
        assert_eq!(section, FlbSectionType::Input);
    }
}
