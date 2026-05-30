use docolint_types::{AnnotatedText, TextSegment};

pub(super) struct RetainedCommentLine<'a> {
    pub text: &'a str,
    pub offset_delta: usize,
    pub needs_join_space: bool,
}

pub(super) fn extract_comment_segments<F>(
    tree: &tree_sitter::Tree,
    content: &str,
    next_unit_id: &mut usize,
    mut visit_comment: F,
) -> AnnotatedText
where
    F: FnMut(tree_sitter::Node<'_>, &str, &mut Vec<TextSegment>, usize),
{
    let mut segments = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    fn walk<F>(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
        next_unit_id: &mut usize,
        last_comment_end: &mut Option<usize>,
        last_comment_unit_id: &mut Option<usize>,
        visit_comment: &mut F,
    ) where
        F: FnMut(tree_sitter::Node<'_>, &str, &mut Vec<TextSegment>, usize),
    {
        let node = cursor.node();
        if is_comment_kind(node.kind()) {
            let start = node.start_byte();
            let raw = std::str::from_utf8(&bytes[start..node.end_byte()]).unwrap_or("");
            let unit_id =
                if should_share_unit_with_previous_comment(bytes, *last_comment_end, start) {
                    let unit_id = (*last_comment_unit_id).expect("last comment unit id missing");
                    append_join_space_to_last_segment(segments, unit_id);
                    unit_id
                } else {
                    let unit_id = *next_unit_id;
                    *next_unit_id += 1;
                    unit_id
                };
            visit_comment(node, raw, segments, unit_id);
            *last_comment_end = Some(node.end_byte());
            *last_comment_unit_id = Some(unit_id);
            return;
        }

        if cursor.goto_first_child() {
            walk(
                cursor,
                bytes,
                segments,
                next_unit_id,
                last_comment_end,
                last_comment_unit_id,
                visit_comment,
            );
            while cursor.goto_next_sibling() {
                walk(
                    cursor,
                    bytes,
                    segments,
                    next_unit_id,
                    last_comment_end,
                    last_comment_unit_id,
                    visit_comment,
                );
            }
            cursor.goto_parent();
        }
    }

    let mut last_comment_end = None;
    let mut last_comment_unit_id = None;
    walk(
        &mut cursor,
        bytes,
        &mut segments,
        next_unit_id,
        &mut last_comment_end,
        &mut last_comment_unit_id,
        &mut visit_comment,
    );
    AnnotatedText { segments }
}

pub(super) fn push_segment(
    segments: &mut Vec<TextSegment>,
    text: String,
    offset: usize,
    unit_id: usize,
) {
    if !text.trim().is_empty() {
        segments.push(TextSegment {
            text,
            is_markup: false,
            offset,
            unit_id,
        });
    }
}

pub(super) fn push_retained_comment_lines(
    segments: &mut Vec<TextSegment>,
    comment_start: usize,
    lines: Vec<RetainedCommentLine<'_>>,
    unit_id: usize,
) {
    for line in lines {
        let mut text = line.text.to_string();
        if line.needs_join_space {
            text.push(' ');
        }

        push_segment(segments, text, comment_start + line.offset_delta, unit_id);
    }
}

pub(super) fn strip_inline_comment_with_offset(raw: &str) -> Option<(String, usize)> {
    let trimmed = raw.trim();

    if let Some(stripped) = trimmed.strip_prefix("//") {
        let leading_ws = stripped.len() - stripped.trim_start().len();
        let text = stripped.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 2 + leading_ws))
        }
    } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
        let text = trimmed
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .trim()
            .to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 2))
        }
    } else {
        None
    }
}

pub(super) fn retained_line_doc_comment_lines<'a>(
    raw: &'a str,
    prefixes: &[&str],
) -> Option<Vec<RetainedCommentLine<'a>>> {
    let mut lines = Vec::new();
    let mut running_offset = 0;

    for line in raw.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let Some((prefix, stripped)) = prefixes.iter().find_map(|prefix| {
            trimmed
                .strip_prefix(prefix)
                .map(|stripped| (*prefix, stripped))
        }) else {
            running_offset += line.len() + 1;
            continue;
        };

        let leading_ws = stripped.len() - stripped.trim_start().len();
        let prose = stripped.trim();
        if !prose.is_empty() {
            lines.push(RetainedCommentLine {
                text: prose,
                offset_delta: running_offset + indent + prefix.len() + leading_ws,
                needs_join_space: false,
            });
        }

        running_offset += line.len() + 1;
    }

    if lines.is_empty() {
        return None;
    }

    let last_index = lines.len() - 1;
    for line in &mut lines[..last_index] {
        line.needs_join_space = true;
    }

    Some(lines)
}

pub(super) fn strip_doc_block_comment_with_offset(raw: &str) -> Option<(String, usize)> {
    let lines = retained_star_prefixed_block_comment_lines(raw, "/**")?;
    let offset = lines.first()?.offset_delta;
    let mut text = String::new();

    for line in &lines {
        text.push_str(line.text);
        if line.needs_join_space {
            text.push(' ');
        }
    }

    Some((text, offset))
}

pub(super) fn retained_doc_block_comment_lines(raw: &str) -> Option<Vec<RetainedCommentLine<'_>>> {
    retained_star_prefixed_block_comment_lines(raw, "/**")
}

pub(super) fn retained_plain_block_comment_lines(
    raw: &str,
) -> Option<Vec<RetainedCommentLine<'_>>> {
    retained_star_prefixed_block_comment_lines(raw, "/*")
}

fn is_comment_kind(kind: &str) -> bool {
    matches!(kind, "comment" | "line_comment" | "block_comment")
}

pub(super) fn should_share_unit_with_previous_comment(
    bytes: &[u8],
    last_comment_end: Option<usize>,
    next_comment_start: usize,
) -> bool {
    let Some(last_comment_end) = last_comment_end else {
        return false;
    };
    let gap = &bytes[last_comment_end..next_comment_start];
    if gap.is_empty() || !gap.iter().all(|byte| byte.is_ascii_whitespace()) {
        return false;
    }

    let newline_count = gap.iter().filter(|&&byte| byte == b'\n').count();
    newline_count <= 1
}

pub(super) fn append_join_space_to_last_segment(segments: &mut [TextSegment], unit_id: usize) {
    let Some(last_segment) = segments.last_mut() else {
        return;
    };
    if last_segment.unit_id != unit_id || last_segment.text.ends_with(char::is_whitespace) {
        return;
    }
    last_segment.text.push(' ');
}

fn retained_star_prefixed_block_comment_lines<'a>(
    raw: &'a str,
    opening_delimiter: &str,
) -> Option<Vec<RetainedCommentLine<'a>>> {
    let inner = raw.strip_prefix(opening_delimiter)?.strip_suffix("*/")?;
    let mut lines = Vec::new();
    let mut running_offset = opening_delimiter.len();

    for line in inner.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let without_star = trimmed.strip_prefix('*').unwrap_or(trimmed);
        let star_delta = usize::from(trimmed.starts_with('*'));
        let leading_ws = without_star.len() - without_star.trim_start().len();
        let prose = without_star.trim();

        if !prose.is_empty() {
            lines.push(RetainedCommentLine {
                text: prose,
                offset_delta: running_offset + indent + star_delta + leading_ws,
                needs_join_space: false,
            });
        }

        running_offset += line.len() + 1;
    }

    if lines.is_empty() {
        return None;
    }

    let last_index = lines.len() - 1;
    for line in &mut lines[..last_index] {
        line.needs_join_space = true;
    }

    Some(lines)
}
