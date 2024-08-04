use std::collections::HashMap;

use tokio::sync::RwLock;
use tower_lsp::{LspService, Server};

use crate::language_server::Backend;

mod assets;
mod completion;
mod document;
mod language_server;

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
