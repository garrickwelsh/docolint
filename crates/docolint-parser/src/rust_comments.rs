use crate::ParserConfig;
use crate::comments::{extract_comment_segments, push_segment, strip_inline_comment_with_offset};
use docolint_types::AnnotatedText;

/// Walk a Rust AST and extract doc comment text as plain segments,
/// everything else as markup.
pub(super) fn extract_rust_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    config: &ParserConfig,
) -> AnnotatedText {
    let bytes = content.as_bytes();

    extract_comment_segments(tree, content, |node, raw, segments| {
        let mut extracted_doc = false;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i as u32).filter(|c| c.kind() == "doc_comment") {
                let start = child.start_byte();
                let text = std::str::from_utf8(&bytes[start..child.end_byte()])
                    .unwrap_or("")
                    .to_string();
                push_segment(segments, text, start);
                extracted_doc = true;
            }
        }

        if extracted_doc || !config.include_inline_comments {
            return;
        }

        let start = node.start_byte();
        if let Some((text, offset_delta)) = strip_inline_comment_with_offset(raw) {
            push_segment(segments, text, start + offset_delta);
        }
    })
}
