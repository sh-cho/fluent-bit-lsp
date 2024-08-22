use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize)]
pub enum FlbSectionType {
    Input,
    Parser,
    MultilineParser,
    Filter,
    Output,
    Custom,

    Other(String),
}

impl FromStr for FlbSectionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "INPUT" => FlbSectionType::Input,
            "PARSER" => FlbSectionType::Parser,
            "MULTILINE_PARSER" => FlbSectionType::MultilineParser,
            "FILTER" => FlbSectionType::Filter,
            "OUTPUT" => FlbSectionType::Output,
            "CUSTOM" => FlbSectionType::Custom,
            _ => FlbSectionType::Other(s.to_string()),
        })
    }
}

impl<'de> Deserialize<'de> for FlbSectionType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FlbSectionType::from_str(&s).map_err(serde::de::Error::custom)
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
            FlbSectionType::Custom => "custom".to_string(),
            FlbSectionType::Other(s) => s.clone(),
        };
        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_name_to_enum() {
        let section: FlbSectionType = str::parse("input").unwrap();
        assert_eq!(section, FlbSectionType::Input);
    }

    // #[test]
    // fn deserialization() {
    //     let section: FlbSectionType = serde_json::from_str("\"INPUT\"").unwrap();
    //     assert_eq!(section, FlbSectionType::Input);
    // }
}
