use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;
use tower_lsp::{lsp_types::MarkupKind, LspService, Server};

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
        map: Arc::new(RwLock::new(HashMap::new())),
        markup_kind: Arc::new(RwLock::new(MarkupKind::PlainText)),
    })
    .finish();

    // TODO: support other commands (e.g. `--version`)

    Server::new(stdin, stdout, socket).serve(service).await;
}
