use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use anyhow::anyhow;
use lazy_regex::{lazy_regex, Lazy, Regex};
use serde::{Deserialize, Deserializer};

use crate::section::FlbSectionType;

/// Represents configuration schema for fluent-bit.
///
/// e.g. [`fluent-bit-schema-3.1.5.json`](https://packages.fluentbit.io/3.1.5/fluent-bit-schema-3.1.5.json)
#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct FlbConfigSchema {
    #[serde(rename = "fluent-bit")]
    pub fluent_bit: FlbInfo,

    pub customs: Vec<FlbPlugin>,
    pub inputs: Vec<FlbPlugin>,
    pub filters: Vec<FlbPlugin>,
    pub outputs: Vec<FlbPlugin>,
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
pub struct FlbInfo {
    /// Version of fluent-bit.
    ///
    /// e.g. `3.1.5`
    pub version: String,

    /// Fluent-bit schema file version.
    ///
    /// Currently only `1` is supported.
    pub schema_version: String,

    /// e.g. `linux`
    pub os: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlbPlugin {
    /// e.g. "input", "output", "filter", ...
    // #[serde(rename = "type")]
    pub type_: FlbSectionType,

    /// e.g. "cpu", "netif", ...
    pub name: String,
    pub description: String,

    /// `properties.options`
    ///
    /// ### Example
    /// ```json
    /// {
    ///     "options": [
    ///         {
    ///             "name": "host",
    ///             "description": "Host Address",
    ///             "default": "",
    ///             "type": "string"
    ///        }
    ///     ],
    ///     // below are used to determine `has_networking` and `has_network_tls`
    ///     "networking": [
    ///         // same as `options`
    ///     ],
    ///     "network_tls": [...]
    /// }
    /// ```
    pub properties: Vec<FlbProperty>,

    pub has_networking: bool,
    pub has_network_tls: bool,
}

impl<'de> Deserialize<'de> for FlbPlugin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Props {
            options: Option<Vec<FlbProperty>>,
            networking: Option<Vec<FlbProperty>>,
            network_tls: Option<Vec<FlbProperty>>,
        }

        #[derive(Deserialize)]
        struct FlbPluginOptions {
            #[serde(rename = "type")]
            type_: FlbSectionType,
            name: String,
            description: String,
            properties: Props,
        }

        let FlbPluginOptions {
            type_,
            name,
            description,
            properties:
                Props {
                    options,
                    networking,
                    network_tls,
                },
        } = FlbPluginOptions::deserialize(deserializer)?;

        Ok(FlbPlugin {
            type_,
            name,
            description,
            properties: options.unwrap_or_default(),
            has_networking: networking.is_some(),
            has_network_tls: network_tls.is_some(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FlbProperty {
    /// ref: [pack_config_map_entry(...)](https://github.com/fluent/fluent-bit/blob/1a1970342bf2097c571d72f2e947f037f6410c4f/src/flb_help.c#L51)
    #[serde(rename = "type")]
    pub type_: FlbPropertyType,
    pub name: String,
    pub description: String,
    // TODO: "" -> None
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlbPropertyType {
    String,
    Integer,
    Boolean,
    Double,
    Size,
    Time,

    /// e.g. `multiple comma delimited strings`
    CommaDelimitedStringsUnlimited,
    /// e.g. `comma delimited strings (minimum %i)`
    CommaDelimitedStringsWithMinimum(u32),

    /// e.g. `multiple space delimited strings`
    SpaceDelimitedStringsUnlimited,
    /// e.g. `space delimited strings (minimum %i)`
    SpaceDelimitedStringsWithMinimum(u32),

    /// e.g. output kafka - `rdkafka.{property}`
    PrefixedString,

    // XXX: do I need this?
    Deprecated,
}

static COMMA_DELIMITED_STRINGS_WITH_MINIMUM_REGEX: Lazy<Regex> =
    lazy_regex!(r"^comma delimited strings \(minimum (?P<minimum>\d+)\)$");

static SPACE_DELIMITED_STRINGS_WITH_MINIMUM_REGEX: Lazy<Regex> =
    lazy_regex!(r"^space delimited strings \(minimum (?P<minimum>\d+)\)$");

impl FromStr for FlbPropertyType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "string" => Ok(FlbPropertyType::String),
            "integer" => Ok(FlbPropertyType::Integer),
            "boolean" => Ok(FlbPropertyType::Boolean),
            "double" => Ok(FlbPropertyType::Double),
            "size" => Ok(FlbPropertyType::Size),
            "time" => Ok(FlbPropertyType::Time),
            "multiple comma delimited strings" => {
                Ok(FlbPropertyType::CommaDelimitedStringsUnlimited)
            }
            "multiple space delimited strings" => {
                Ok(FlbPropertyType::SpaceDelimitedStringsUnlimited)
            }
            "prefixed string" => Ok(FlbPropertyType::PrefixedString),
            "deprecated" => Ok(FlbPropertyType::Deprecated),
            _ => {
                if let Some(captures) = COMMA_DELIMITED_STRINGS_WITH_MINIMUM_REGEX.captures(s) {
                    let minimum = captures["minimum"].parse()?;
                    return Ok(FlbPropertyType::CommaDelimitedStringsWithMinimum(minimum));
                } else if let Some(captures) =
                    SPACE_DELIMITED_STRINGS_WITH_MINIMUM_REGEX.captures(s)
                {
                    let minimum = captures["minimum"].parse()?;
                    return Ok(FlbPropertyType::SpaceDelimitedStringsWithMinimum(minimum));
                }

                Err(anyhow!("Unknown property type: {}", s))
            }
        }
    }
}

impl<'de> Deserialize<'de> for FlbPropertyType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FlbPropertyType::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Display for FlbPropertyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            FlbPropertyType::String => "string".to_string(),
            FlbPropertyType::Integer => "integer".to_string(),
            FlbPropertyType::Boolean => "boolean".to_string(),
            FlbPropertyType::Double => "double".to_string(),
            FlbPropertyType::Size => "size".to_string(),
            FlbPropertyType::Time => "time".to_string(),
            FlbPropertyType::CommaDelimitedStringsUnlimited => {
                "multiple comma delimited strings".to_string()
            }
            FlbPropertyType::CommaDelimitedStringsWithMinimum(minimum) => {
                format!("comma delimited strings (minimum {})", minimum)
            }
            FlbPropertyType::SpaceDelimitedStringsUnlimited => {
                "multiple space delimited strings".to_string()
            }
            FlbPropertyType::SpaceDelimitedStringsWithMinimum(minimum) => {
                format!("space delimited strings (minimum {})", minimum)
            }
            FlbPropertyType::PrefixedString => "prefixed string".to_string(),
            FlbPropertyType::Deprecated => "deprecated".to_string(),
        };

        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flb_plugin_deserialize() {
        let plugin: FlbPlugin = serde_json::from_str(
            r#"{
                "options": [
                    {
                        "name": "host",
                        "description": "Host Address",
                        "default": "",
                        "type": "string"
                    }
                ],
                "networking": [
                    {
                        "name": "net.dns.mode",
                        "description": "Select the primary DNS connection type (TCP or UDP)",
                        "default": null,
                        "type": "string"
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(plugin, FlbPlugin {
            type_: FlbSectionType::Custom,
            name: "custom".to_string(),
            description: "custom".to_string(),
            properties: vec![FlbProperty {
                type_: FlbPropertyType::String,
                name: "host".to_string(),
                description: "Host Address".to_string(),
                default: Some("".to_string())
            }],
            has_networking: true,
            has_network_tls: false
        });
    }

    #[test]
    fn flb_property_deserialize() {
        let property: FlbProperty = serde_json::from_str(
            r#"{
                "type": "space delimited strings (minimum 3)",
                "name": "name",
                "description": "desc",
                "default": "abc def ghi"
            }"#,
        )
        .unwrap();

        assert_eq!(property, FlbProperty {
            type_: FlbPropertyType::SpaceDelimitedStringsWithMinimum(3),
            name: "name".to_string(),
            description: "desc".to_string(),
            default: Some("abc def ghi".to_string())
        });
    }
}
