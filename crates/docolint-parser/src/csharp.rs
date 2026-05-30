use crate::ParserConfig;
use crate::comments::{
    append_join_space_to_last_segment, extract_comment_segments, push_retained_comment_lines,
    push_segment, retained_doc_block_comment_lines, retained_line_doc_comment_lines,
    strip_doc_block_comment_with_offset, strip_inline_comment_with_offset,
};
use docolint_types::AnnotatedText;

/// Walk a C# AST and extract doc comment text as plain segments.
/// C# `comment` nodes contain the raw `/// ...` or `/** ... */` text.
pub(super) fn extract_csharp_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    config: &ParserConfig,
    next_unit_id: &mut usize,
) -> AnnotatedText {
    let mut last_triple_slash_row = None;
    let mut last_triple_slash_unit_id = None;

    extract_comment_segments(
        tree,
        content,
        next_unit_id,
        |node, raw, segments, unit_id| {
            let start = node.start_byte();
            if raw.contains('\n') && raw.trim_start().starts_with("/**") {
                if let Some(lines) = retained_doc_block_comment_lines(raw) {
                    push_retained_comment_lines(segments, start, lines, unit_id);
                    return;
                }
            }

            if raw.trim_start().starts_with("///")
                && last_triple_slash_row
                    .filter(|last_row| *last_row + 1 == node.start_position().row)
                    .is_some()
            {
                let shared_unit_id =
                    last_triple_slash_unit_id.expect("last C# doc unit id missing");
                if shared_unit_id != unit_id {
                    append_join_space_to_last_segment(segments, shared_unit_id);
                }

                if let Some(lines) = retained_line_doc_comment_lines(raw, &["///"]) {
                    push_retained_comment_lines(segments, start, lines, shared_unit_id);
                    last_triple_slash_row = Some(node.start_position().row);
                    last_triple_slash_unit_id = Some(shared_unit_id);
                    return;
                }
            }

            if let Some((text, offset_delta)) =
                extract_csharp_comment(raw, config.include_inline_comments)
            {
                let effective_unit_id = if raw.trim_start().starts_with("///") {
                    last_triple_slash_row = Some(node.start_position().row);
                    last_triple_slash_unit_id = Some(unit_id);
                    unit_id
                } else {
                    unit_id
                };
                push_segment(segments, text, start + offset_delta, effective_unit_id);
            }
        },
    )
}

fn extract_csharp_comment(raw: &str, include_inline: bool) -> Option<(String, usize)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("///") {
        let lines = retained_line_doc_comment_lines(raw, &["///"])?;
        let offset = lines.first()?.offset_delta;
        let mut text = String::new();

        for line in &lines {
            text.push_str(line.text);
            if line.needs_join_space {
                text.push(' ');
            }
        }

        Some((text, offset))
    } else if trimmed.starts_with("/**") {
        strip_doc_block_comment_with_offset(raw)
    } else if include_inline {
        strip_inline_comment_with_offset(raw)
    } else {
        None
    }
}
