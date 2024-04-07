use std::cell::RefCell;
use ropey::Rope;
use tokio::time::Instant;
use tracing::info;
use tree_sitter::{Parser, Point, Tree};

// pub struct FlbParser {
//     tree: Tree,
//     source_code: String,
// }
//
// impl FlbParser {
//     pub fn new(source_code: &str, old_tree: Option<&Tree>) -> Self {
//         let mut parser = Parser::new();
//         parser.set_language(tree_sitter_fluentbit::language())
//             .expect("Error loading fluent-bit grammar");
//
//         let tree = parser.parse(&source_code, None).unwrap();
//
//         FlbParser { tree, source_code: source_code.to_string() }
//     }
//
//     pub fn get_node_at_point(&self, point: &Point) -> Option<Node> {
//         self.tree.root_node().descendant_for_point_range(*point, *point)
//     }
// }

thread_local! {
    static FLB_PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

pub fn parse(source_code: &Rope, old_tree: Option<&Tree>) -> Tree {
    let t = Instant::now();
    let tree = FLB_PARSER
        .with(|parser| {
            if parser.borrow().language().is_none() {
                parser.borrow_mut().set_language(tree_sitter_fluentbit::language())
                    .expect("Error loading fluent-bit grammar");
            }

            parser.borrow_mut().parse_with(
                &mut |byte_offset: usize, _: Point| {
                    if byte_offset > source_code.len_bytes() {
                        ""
                    } else {
                        source_code.byte_slice(byte_offset..)
                            .chunks()
                            .next()
                            .unwrap_or("")
                    }
                },
                old_tree,
            )
        })
        .unwrap();

    info!("Parsed in {:?}", t.elapsed());
    tree
}
