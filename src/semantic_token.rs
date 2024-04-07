use std::any::Any;
use tower_lsp::lsp_types::{SemanticTokenModifier, SemanticTokenType};
use tree_sitter::Node;

pub fn token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::COMMENT,
        SemanticTokenType::KEYWORD,
        SemanticTokenType::MACRO,
        SemanticTokenType::TYPE,
        SemanticTokenType::STRING,
    ]
}

pub fn token_modifiers() -> Vec<SemanticTokenModifier> {
    vec![]
}

// TODO:
// pub fn tree_sitter_token_to_semantic_token(
//     node: &Node,
// ) {
//     node.kind_id()
// }
