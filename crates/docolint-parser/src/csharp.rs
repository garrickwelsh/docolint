use crate::ParserConfig;
use crate::comments::{
    append_join_space_to_last_segment, push_retained_comment_lines, push_segment,
    retained_doc_block_comment_lines, retained_line_doc_comment_lines,
    should_share_unit_with_previous_comment, strip_doc_block_comment_with_offset,
    strip_inline_comment_with_offset,
};
use crate::csharp_xml;
use docolint_types::AnnotatedText;

/// Walk a C# AST and extract doc comment text as plain segments.
/// C# `comment` nodes contain the raw `/// ...` or `/** ... */` text.
pub(super) fn extract_csharp_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    config: &ParserConfig,
    next_unit_id: &mut usize,
) -> AnnotatedText {
    let mut segments = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();
    let mut last_triple_slash_row = None;
    let mut last_triple_slash_unit_id = None;
    let mut last_comment_end = None;
    let mut last_comment_unit_id = None;
    let mut pending_line_docs: Vec<(usize, &str)> = Vec::new();

    fn flush_pending_line_docs(
        segments: &mut Vec<docolint_types::TextSegment>,
        pending_line_docs: &mut Vec<(usize, &str)>,
        pending_unit_id: &mut Option<usize>,
        next_unit_id: &mut usize,
    ) {
        if pending_line_docs.is_empty() {
            return;
        }

        let unit_id = pending_unit_id
            .take()
            .expect("pending C# doc unit id missing");
        let should_parse_xml = pending_line_docs
            .iter()
            .any(|(_, raw)| raw.contains('<') || raw.contains('&'));
        if should_parse_xml
            && let Some(xml_segments) =
                csharp_xml::extract_line_doc_group(pending_line_docs, unit_id, next_unit_id)
        {
            segments.extend(xml_segments);
        } else {
            for (index, (start, raw)) in pending_line_docs.iter().enumerate() {
                if index > 0 {
                    append_join_space_to_last_segment(segments, unit_id);
                }
                if let Some(lines) = retained_line_doc_comment_lines(raw, &["///"]) {
                    push_retained_comment_lines(segments, *start, lines, unit_id);
                }
            }
        }

        pending_line_docs.clear();
    }

    fn walk<'a>(
        cursor: &mut tree_sitter::TreeCursor<'a>,
        bytes: &'a [u8],
        config: &ParserConfig,
        segments: &mut Vec<docolint_types::TextSegment>,
        next_unit_id: &mut usize,
        last_comment_end: &mut Option<usize>,
        last_comment_unit_id: &mut Option<usize>,
        last_triple_slash_row: &mut Option<usize>,
        last_triple_slash_unit_id: &mut Option<usize>,
        pending_line_docs: &mut Vec<(usize, &'a str)>,
    ) {
        let node = cursor.node();
        if matches!(node.kind(), "comment" | "line_comment" | "block_comment") {
            let start = node.start_byte();
            let raw = std::str::from_utf8(&bytes[start..node.end_byte()]).unwrap_or("");
            let is_line_doc = raw.trim_start().starts_with("///");
            let row = node.start_position().row;
            let continues_pending = is_line_doc
                && last_triple_slash_row
                    .filter(|last_row| *last_row + 1 == row)
                    .is_some();

            if !continues_pending {
                flush_pending_line_docs(
                    segments,
                    pending_line_docs,
                    last_triple_slash_unit_id,
                    next_unit_id,
                );
                *last_triple_slash_row = None;
            }

            let unit_id = if continues_pending {
                last_triple_slash_unit_id.expect("last C# doc unit id missing")
            } else if should_share_unit_with_previous_comment(bytes, *last_comment_end, start) {
                let unit_id = (*last_comment_unit_id).expect("last comment unit id missing");
                append_join_space_to_last_segment(segments, unit_id);
                unit_id
            } else {
                let unit_id = *next_unit_id;
                *next_unit_id += 1;
                unit_id
            };

            if is_line_doc {
                pending_line_docs.push((start, raw));
                *last_triple_slash_row = Some(row);
                *last_triple_slash_unit_id = Some(unit_id);
            } else if raw.trim_start().starts_with("/**") {
                let should_parse_xml = raw.contains('<') || raw.contains('&');
                if should_parse_xml
                    && let Some(xml_segments) =
                        csharp_xml::extract_block_doc(start, raw, unit_id, next_unit_id)
                {
                    segments.extend(xml_segments);
                } else if let Some(lines) = retained_doc_block_comment_lines(raw) {
                    push_retained_comment_lines(segments, start, lines, unit_id);
                }
            } else if let Some((text, offset_delta)) =
                extract_csharp_comment(raw, config.include_inline_comments)
            {
                push_segment(segments, text, start + offset_delta, unit_id);
            }

            *last_comment_end = Some(node.end_byte());
            *last_comment_unit_id = Some(unit_id);
            return;
        }

        if cursor.goto_first_child() {
            walk(
                cursor,
                bytes,
                config,
                segments,
                next_unit_id,
                last_comment_end,
                last_comment_unit_id,
                last_triple_slash_row,
                last_triple_slash_unit_id,
                pending_line_docs,
            );
            while cursor.goto_next_sibling() {
                walk(
                    cursor,
                    bytes,
                    config,
                    segments,
                    next_unit_id,
                    last_comment_end,
                    last_comment_unit_id,
                    last_triple_slash_row,
                    last_triple_slash_unit_id,
                    pending_line_docs,
                );
            }
            cursor.goto_parent();
        }
    }

    walk(
        &mut cursor,
        bytes,
        config,
        &mut segments,
        next_unit_id,
        &mut last_comment_end,
        &mut last_comment_unit_id,
        &mut last_triple_slash_row,
        &mut last_triple_slash_unit_id,
        &mut pending_line_docs,
    );
    flush_pending_line_docs(
        &mut segments,
        &mut pending_line_docs,
        &mut last_triple_slash_unit_id,
        next_unit_id,
    );

    AnnotatedText { segments }
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
