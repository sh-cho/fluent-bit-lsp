//! https://gist.github.com/rojas-diego/04d9c4e3fff5f8374f29b9b738d541ef

use ropey::{Rope, RopeSlice};
use thiserror::Error;
use tower_lsp::lsp_types::{Position, TextDocumentContentChangeEvent};
use tree_sitter::{InputEdit, Parser, Point, Tree};

pub struct TextDocument {
    pub rope: Rope,
    pub tree: Option<Tree>,
    parser: Parser,
}

#[derive(Error, Debug)]
pub enum DocumentError {
    #[error("position {0}:{1} is out of bounds")]
    PositionOutOfBounds(u32, u32),
}

#[derive(Clone, Debug, Copy)]
/// We redeclare this enum here because the `lsp_types` crate exports a Cow
/// type that is unconvenient to deal with.
pub enum PositionEncodingKind {
    #[allow(dead_code)]
    UTF8,
    UTF16,
    #[allow(dead_code)]
    UTF32,
}

impl TextDocument {
    /// Creates a new document from the given text and language id. It creates
    /// a rope, parser and syntax tree from the text.
    pub fn new(text: &str) -> Self {
        let rope = Rope::from_str(text);
        let mut parser = Parser::new();

        let language = tree_sitter_fluentbit::language();

        parser
            .set_language(&language)
            .expect("set parser language should always succeed");

        let tree = parser
            .parse(text, None)
            .expect("parse should always return a tree when the language was set and no timeout was specified");

        Self {
            rope,
            tree: Some(tree),
            parser,
        }
    }

    /// Apply a change to the document.
    pub fn apply_content_change(
        &mut self,
        change: &TextDocumentContentChangeEvent,
        position_encoding: PositionEncodingKind,
    ) -> Result<(), DocumentError> {
        match change.range {
            Some(range) => {
                assert!(
                    range.start.line < range.end.line
                        || (range.start.line == range.end.line
                            && range.start.character <= range.end.character)
                );

                let same_line = range.start.line == range.end.line;
                let same_character = range.start.character == range.end.character;

                let change_start_line_cu_idx = range.start.character as usize;
                let change_end_line_cu_idx = range.end.character as usize;

                // 1. Get the line at which the change starts.
                let change_start_line_idx = range.start.line as usize;
                let change_start_line = match self.rope.get_line(change_start_line_idx) {
                    Some(line) => line,
                    None => {
                        return Err(DocumentError::PositionOutOfBounds(
                            range.start.line,
                            range.start.character,
                        ))
                    }
                };

                // 2. Get the line at which the change ends. (Small optimization
                // where we first check whether start and end line are the
                // same O(log N) lookup. We repeat this all throughout this
                // function).
                let change_end_line_idx = range.end.line as usize;
                let change_end_line = match same_line {
                    true => change_start_line,
                    false => match self.rope.get_line(change_end_line_idx) {
                        Some(line) => line,
                        None => {
                            return Err(DocumentError::PositionOutOfBounds(
                                range.end.line,
                                range.end.character,
                            ));
                        }
                    },
                };

                fn compute_char_idx(
                    position_encoding: PositionEncodingKind,
                    position: &Position,
                    slice: &RopeSlice,
                ) -> Result<usize, DocumentError> {
                    match position_encoding {
                        PositionEncodingKind::UTF8 => {
                            slice.try_byte_to_char(position.character as usize)
                        }
                        PositionEncodingKind::UTF16 => {
                            slice.try_utf16_cu_to_char(position.character as usize)
                        }
                        PositionEncodingKind::UTF32 => Ok(position.character as usize),
                    }
                    .map_err(|_| {
                        DocumentError::PositionOutOfBounds(position.line, position.character)
                    })
                }

                // 3. Compute the character offset into the start/end line where
                // the change starts/ends.
                let change_start_line_char_idx =
                    compute_char_idx(position_encoding, &range.start, &change_start_line)?;
                let change_end_line_char_idx = match same_line && same_character {
                    true => change_start_line_char_idx,
                    false => compute_char_idx(position_encoding, &range.end, &change_end_line)?,
                };

                // 4. Compute the character and byte offset into the document
                // where the change starts/ends.
                let change_start_doc_char_idx =
                    self.rope.line_to_char(change_start_line_idx) + change_start_line_char_idx;
                let change_end_doc_char_idx = match same_line && same_character {
                    true => change_start_doc_char_idx,
                    false => self.rope.line_to_char(change_end_line_idx) + change_end_line_char_idx,
                };
                let change_start_doc_byte_idx = self.rope.char_to_byte(change_start_doc_char_idx);
                let change_end_doc_byte_idx = match same_line && same_character {
                    true => change_start_doc_byte_idx,
                    false => self.rope.char_to_byte(change_end_doc_char_idx),
                };

                // 5. Compute the byte offset into the start/end line where the
                // change starts/ends. Required for tree-sitter.
                let change_start_line_byte_idx = match position_encoding {
                    PositionEncodingKind::UTF8 => change_start_line_cu_idx,
                    PositionEncodingKind::UTF16 => {
                        change_start_line.char_to_utf16_cu(change_start_line_char_idx)
                    }
                    PositionEncodingKind::UTF32 => change_start_line_char_idx,
                };
                let change_end_line_byte_idx = match same_line && same_character {
                    true => change_start_line_byte_idx,
                    false => match position_encoding {
                        PositionEncodingKind::UTF8 => change_end_line_cu_idx,
                        PositionEncodingKind::UTF16 => {
                            change_end_line.char_to_utf16_cu(change_end_line_char_idx)
                        }
                        PositionEncodingKind::UTF32 => change_end_line_char_idx,
                    },
                };

                self.rope
                    .remove(change_start_doc_char_idx..change_end_doc_char_idx);
                self.rope.insert(change_start_doc_char_idx, &change.text);

                if let Some(tree) = &mut self.tree {
                    // 6. Compute the byte index into the new end line where the
                    // change ends. Required for tree-sitter.
                    let change_new_end_line_idx = self
                        .rope
                        .byte_to_line(change_start_doc_byte_idx + change.text.len());
                    let change_new_end_line_byte_idx =
                        change_start_doc_byte_idx + change.text.len();

                    // 7. Construct the tree-sitter edit. We stay mindful that
                    // tree-sitter Point::column is a byte offset.
                    let edit = InputEdit {
                        start_byte: change_start_doc_byte_idx,
                        old_end_byte: change_end_doc_byte_idx,
                        new_end_byte: change_start_doc_byte_idx + change.text.len(),
                        start_position: Point {
                            row: change_start_line_idx,
                            column: change_start_line_byte_idx,
                        },
                        old_end_position: Point {
                            row: change_end_line_idx,
                            column: change_end_line_byte_idx,
                        },
                        new_end_position: Point {
                            row: change_new_end_line_idx,
                            column: change_new_end_line_byte_idx,
                        },
                    };

                    tree.edit(&edit);

                    self.tree = Some(self
                        .parser
                        .parse(self.rope.to_string(), Some(tree))
                        .expect("parse should always return a tree when the language was set and no timeout was specified"));
                }

                return Ok(());
            }
            None => {
                self.rope = Rope::from_str(&change.text);
                self.tree = self.parser.parse(&change.text, None);

                return Ok(());
            }
        }
    }
}

#[cfg(test)]
mod test {
    use tree_sitter::Node;

    use super::*;

    macro_rules! new_change {
        ($start_line:expr, $start_char:expr, $end_line:expr, $end_char:expr, $text:expr) => {
            &TextDocumentContentChangeEvent {
                range: Some(Range::new(
                    Position::new($start_line as u32, $start_char as u32),
                    Position::new($end_line as u32, $end_char as u32),
                )),
                range_length: None,
                text: $text.to_string(),
            }
        };
    }

    // #[test]
    // fn test_text_document_apply_content_change() {
    //     let mut rope = Rope::from_str("🤗 Hello 🤗\nABC 🇫🇷\n world!");
    //     let mut doc = TextDocument::new(LanguageId::Unknown, &rope.to_string());
    //
    //     doc.apply_content_change(new_change!(0, 0, 0, 3, ""), PositionEncodingKind::UTF16)
    //         .unwrap();
    //     rope = Rope::from_str("Hello 🤗\nABC 🇫🇷\n world!");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     doc.apply_content_change(
    //         new_change!(1, 4 + "🇫🇷".len(), 1, 4 + "🇫🇷".len(), " DEF"),
    //         PositionEncodingKind::UTF8,
    //     )
    //         .unwrap();
    //     rope = Rope::from_str("Hello 🤗\nABC 🇫🇷 DEF\n world!");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     doc.apply_content_change(
    //         new_change!(1, 0, 1, 4 + "🇫🇷".chars().count() + 4, ""),
    //         PositionEncodingKind::UTF32,
    //     )
    //         .unwrap();
    //     rope = Rope::from_str("Hello 🤗\n\n world!");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     doc.apply_content_change(new_change!(1, 0, 1, 1, ""), PositionEncodingKind::UTF16)
    //         .unwrap();
    //     rope = Rope::from_str("Hello 🤗\n world!");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     doc.apply_content_change(new_change!(0, 5, 1, 1, "，"), PositionEncodingKind::UTF16)
    //         .unwrap();
    //     rope = Rope::from_str("Hello，world!");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     doc.apply_content_change(
    //         new_change!(0, 0, 0, rope.len_utf16_cu(), ""),
    //         PositionEncodingKind::UTF16,
    //     )
    //         .unwrap();
    //     assert_eq!(doc.rope.to_string(), "");
    // }
    //
    // #[test]
    // fn test_text_document_apply_content_change_no_range() {
    //     let mut rope = Rope::from_str(
    //         "let a = '🥸 你好';\rfunction helloWorld() { return '🤲🏿'; }\nlet b = 'Hi, 😊';",
    //     );
    //     let mut doc = TextDocument::new(LanguageId::JavaScript, &rope.to_string());
    //     let mut parser = Parser::new();
    //
    //     parser
    //         .set_language(tree_sitter_javascript::language())
    //         .unwrap();
    //
    //     assert!(doc.apply_content_change(
    //         &TextDocumentContentChangeEvent {
    //             range: None,
    //             range_length: None,
    //             text: "let a = '🥸 你好';\rfunction helloWorld() { return '🤲🏿'; }\nlet b = 'Hi, 😊';".to_owned(),
    //         },
    //         PositionEncodingKind::UTF16,
    //     ).is_ok());
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     let tree = parser.parse(&rope.to_string(), None).unwrap();
    //
    //     assert!(nodes_are_equal_recursive(
    //         &doc.tree.as_ref().unwrap().root_node(),
    //         &tree.root_node()
    //     ));
    //
    //     assert!(doc
    //         .apply_content_change(
    //             &TextDocumentContentChangeEvent {
    //                 range: None,
    //                 range_length: None,
    //                 text: "let a = '🥸 你好，😊';".to_owned(),
    //             },
    //             PositionEncodingKind::UTF16,
    //         )
    //         .is_ok());
    //     rope = Rope::from_str("let a = '🥸 你好，😊';");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     let tree = parser.parse(&rope.to_string(), None).unwrap();
    //
    //     assert!(nodes_are_equal_recursive(
    //         &doc.tree.as_ref().unwrap().root_node(),
    //         &tree.root_node()
    //     ));
    // }
    //
    // #[test]
    // fn test_text_document_apply_content_change_bounds() {
    //     let rope = Rope::from_str("");
    //     let mut doc = TextDocument::new(LanguageId::Unknown, &rope.to_string());
    //
    //     assert!(doc
    //         .apply_content_change(new_change!(0, 0, 0, 1, ""), PositionEncodingKind::UTF16)
    //         .is_err());
    //
    //     assert!(doc
    //         .apply_content_change(new_change!(1, 0, 1, 0, ""), PositionEncodingKind::UTF16)
    //         .is_err());
    //
    //     assert!(doc
    //         .apply_content_change(new_change!(0, 0, 0, 0, "🤗"), PositionEncodingKind::UTF16)
    //         .is_ok());
    //     let rope = Rope::from_str("🤗");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     assert!(doc
    //         .apply_content_change(
    //             new_change!(0, rope.len_utf16_cu(), 0, rope.len_utf16_cu(), "\r\n"),
    //             PositionEncodingKind::UTF16
    //         )
    //         .is_ok());
    //     let rope = Rope::from_str("🤗\r\n");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     assert!(doc
    //         .apply_content_change(
    //             new_change!(0, '🤗'.len_utf16(), 0, '🤗'.len_utf16(), "\n"),
    //             PositionEncodingKind::UTF16
    //         )
    //         .is_ok());
    //     let rope = Rope::from_str("🤗\n\r\n");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    //
    //     assert!(doc
    //         .apply_content_change(
    //             new_change!(0, '🤗'.len_utf16(), 2, 0, ""),
    //             PositionEncodingKind::UTF16
    //         )
    //         .is_ok());
    //     let rope = Rope::from_str("🤗");
    //     assert_eq!(doc.rope.to_string(), rope.to_string());
    // }
    //
    // #[test]
    // // Ensure that the three stays consistent across updates.
    // fn test_document_update_tree_consistency_easy() {
    //     let a = "let a = '你好';\rlet b = 'Hi, 😊';";
    //
    //     let mut document = TextDocument::new(LanguageId::JavaScript, a);
    //
    //     document
    //         .apply_content_change(new_change!(0, 9, 0, 11, "𐐀"), PositionEncodingKind::UTF16)
    //         .unwrap();
    //
    //     let b = "let a = '𐐀';\rlet b = 'Hi, 😊';";
    //
    //     assert_eq!(document.rope.to_string(), b);
    //
    //     let mut parser = Parser::new();
    //
    //     parser
    //         .set_language(tree_sitter_javascript::language())
    //         .unwrap();
    //
    //     let b_tree = parser.parse(b, None).unwrap();
    //
    //     assert!(nodes_are_equal_recursive(
    //         &document.tree.unwrap().root_node(),
    //         &b_tree.root_node()
    //     ));
    // }
    //
    // #[test]
    // fn test_document_update_tree_consistency_medium() {
    //     let a = "let a = '🥸 你好';\rfunction helloWorld() { return '🤲🏿'; }\nlet b = 'Hi, 😊';";
    //
    //     let mut document = TextDocument::new(LanguageId::JavaScript, a);
    //
    //     document
    //         .apply_content_change(new_change!(0, 14, 2, 13, "，"), PositionEncodingKind::UTF16)
    //         .unwrap();
    //
    //     let b = "let a = '🥸 你好，😊';";
    //
    //     assert_eq!(document.rope.to_string(), b);
    //
    //     let mut parser = Parser::new();
    //
    //     parser
    //         .set_language(tree_sitter_javascript::language())
    //         .unwrap();
    //
    //     let b_tree = parser.parse(b, None).unwrap();
    //
    //     assert!(nodes_are_equal_recursive(
    //         &document.tree.unwrap().root_node(),
    //         &b_tree.root_node()
    //     ));
    // }
    //
    // #[test]
    // /// I wrote this test because I was unsure whether tree-sitter's Point
    // /// struct represents a byte or character offset. Turns out it's a byte
    // /// offset.
    // fn test_tree_sitter_point() {
    //     let mut parser = Parser::new();
    //
    //     let language = tree_sitter_javascript::language();
    //
    //     parser.set_language(language).unwrap();
    //
    //     let source_code = "let a = \"你好\";\nlet x = 'Hi, 😊';";
    //     let parsed = parser.parse(source_code, None).unwrap();
    //     let root_node = parsed.root_node();
    //
    //     // Expect the '🤗' to be in "program.lexical_declaration.variable_declarator.string.string_fragment"
    //     // and to take up 4 bytes.
    //     let mut node = root_node
    //         .descendant_for_point_range(
    //             tree_sitter::Point { row: 1, column: 9 },
    //             tree_sitter::Point { row: 1, column: 17 },
    //         )
    //         .unwrap();
    //
    //     assert_eq!(node.kind(), "string_fragment");
    //
    //     // Assert the ancestor chain
    //     node = node.parent().unwrap();
    //     assert_eq!(node.kind(), "string");
    //     node = node.parent().unwrap();
    //     assert_eq!(node.kind(), "variable_declarator");
    //     node = node.parent().unwrap();
    //     assert_eq!(node.kind(), "lexical_declaration");
    //     node = node.parent().unwrap();
    //     assert_eq!(node.kind(), "program");
    //     assert_eq!(node.parent(), None);
    //
    //     // Conversely, we check that ropey effectively uses character offsets.
    //     let rope = Rope::from_str(source_code);
    //
    //     let second_line = rope.line(1);
    //     assert_eq!(second_line.len_chars(), 16);
    //     assert_eq!(second_line.len_bytes(), 19);
    //     assert_eq!(second_line.chars().nth(13), Some('😊'));
    // }

    #[test]
    /// I wrote this test to better understand ropey's handling of ranges.
    fn test_rope_remove_and_insert() {
        let source_code = "let x = 'Hi, 😊';".to_owned();
        let mut rope = Rope::from_str(&source_code);

        assert_eq!(rope.len_chars(), source_code.chars().count());

        rope.remove(0..source_code.chars().count());
        assert_eq!(rope.len_chars(), 0);

        rope.insert(0, &source_code);
        assert_eq!(rope.len_chars(), source_code.chars().count());

        rope.remove(0..source_code.chars().count() - 1);
        assert_eq!(rope.len_chars(), 1);

        rope.remove(0..1);
        assert_eq!(rope.len_chars(), 0);

        rope = Rope::from_str("");

        assert_eq!(rope.lines().nth(0), Some(rope.slice(0..0)));
    }

    fn nodes_are_equal_recursive(node1: &Node, node2: &Node) -> bool {
        if node1.kind() != node2.kind() {
            return false;
        }

        if node1.start_byte() != node2.start_byte() {
            return false;
        }

        if node1.end_byte() != node2.end_byte() {
            return false;
        }

        if node1.start_position() != node2.start_position() {
            return false;
        }

        if node1.end_position() != node2.end_position() {
            return false;
        }

        if node1.child_count() != node2.child_count() {
            return false;
        }

        for i in 0..node1.child_count() {
            let child1 = node1.child(i).unwrap();
            let child2 = node2.child(i).unwrap();

            if !nodes_are_equal_recursive(&child1, &child2) {
                return false;
            }
        }

        true
    }
}
