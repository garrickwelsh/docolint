use crate::ParserConfig;
use crate::comments::{
    extract_comment_segments, push_retained_comment_lines, push_segment,
    retained_line_doc_comment_lines, strip_doc_block_comment_with_offset,
    strip_inline_comment_with_offset,
};
use docolint_types::AnnotatedText;

/// Walk a Rust AST and extract doc comment text as plain segments,
/// everything else as markup.
pub(super) fn extract_rust_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    config: &ParserConfig,
    next_unit_id: &mut usize,
) -> AnnotatedText {
    let mut last_doc_comment_row = None;
    let mut last_doc_unit_id = None;

    extract_comment_segments(
        tree,
        content,
        next_unit_id,
        |node, raw, segments, unit_id| {
            let start = node.start_byte();
            if let Some(lines) = retained_line_doc_comment_lines(raw, &["///", "//!"]) {
                let current_row = node.start_position().row;
                let shared_unit_id = last_doc_comment_row
                    .filter(|last_row| *last_row + 1 == current_row)
                    .and(last_doc_unit_id);
                let unit_id = if let Some(shared_unit_id) = shared_unit_id {
                    if let Some(last_segment) = segments.last_mut() {
                        if !last_segment.text.ends_with(char::is_whitespace) {
                            last_segment.text.push(' ');
                        }
                    }
                    shared_unit_id
                } else {
                    unit_id
                };

                push_retained_comment_lines(segments, start, lines, unit_id);
                last_doc_comment_row = Some(current_row);
                last_doc_unit_id = Some(unit_id);
                return;
            }

            if let Some((text, offset_delta)) = strip_doc_block_comment_with_offset(raw) {
                push_segment(segments, text, start + offset_delta, unit_id);
                return;
            }

            if !config.include_inline_comments {
                return;
            }

            if let Some((text, offset_delta)) = strip_inline_comment_with_offset(raw) {
                push_segment(segments, text, start + offset_delta, unit_id);
            }
        },
    )
}
