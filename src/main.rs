use std::collections::HashMap;
use std::str::FromStr;

use tokio::sync::RwLock;
use tower_lsp::{LspService, Server};

use crate::language_server::Backend;

mod assets;
mod completion;
mod document;
mod language_server;

#[derive(Debug, PartialEq)]
enum SectionType {
    Input,
    Parser,
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
            "FILTER" => SectionType::Filter,
            "OUTPUT" => SectionType::Output,
            _ => SectionType::Other(s.to_string()),
        })
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        map: RwLock::new(HashMap::new()),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
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
