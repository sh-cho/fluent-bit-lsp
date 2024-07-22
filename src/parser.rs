use tree_sitter::{Parser, Tree};

pub struct MyParser {
    pub(crate) parser: Parser,  // Should I hold parser?
    pub(crate) tree: Tree,
    pub(crate) source_code: String,  // TODO: &str ?
}

impl MyParser {
    pub fn new(source_code: &str) -> Self {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fluentbit::language()).expect("Error loading fluentbit grammar");

        let tree = parser.parse(source_code, None).unwrap();

        Self {
            parser,
            tree,
            source_code: source_code.to_string(),
        }
    }

    pub fn update(&mut self, source_code: &str) {
        let new_tree = self.parser.parse(source_code, Some(&self.tree)).unwrap();

        self.tree = new_tree;
        self.source_code = source_code.to_string();
    }
}

#[cfg(test)]
mod tests {
    use tree_sitter::Parser;

    #[tokio::test]
    async fn parser() {
        let source_code = "
    [INPUT]
    KEY1    VALUE1
    ";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_fluentbit::language()).unwrap();

        let tree = parser.parse(source_code, None).unwrap();

        let root_node = tree.root_node();

        // TODO: use NODE_TYPES?
        assert_eq!(root_node.kind(), "config");
    }
}
