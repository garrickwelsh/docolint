use docolint_types::{AnnotatedText, TextSegment};

pub(super) fn extract_comment_segments<F>(
    tree: &tree_sitter::Tree,
    content: &str,
    mut visit_comment: F,
) -> AnnotatedText
where
    F: FnMut(tree_sitter::Node<'_>, &str, &mut Vec<TextSegment>),
{
    let mut segments = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    fn walk<F>(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
        visit_comment: &mut F,
    ) where
        F: FnMut(tree_sitter::Node<'_>, &str, &mut Vec<TextSegment>),
    {
        let node = cursor.node();
        if is_comment_kind(node.kind()) {
            let start = node.start_byte();
            let raw = std::str::from_utf8(&bytes[start..node.end_byte()]).unwrap_or("");
            visit_comment(node, raw, segments);
            return;
        }

        if cursor.goto_first_child() {
            walk(cursor, bytes, segments, visit_comment);
            while cursor.goto_next_sibling() {
                walk(cursor, bytes, segments, visit_comment);
            }
            cursor.goto_parent();
        }
    }

    walk(&mut cursor, bytes, &mut segments, &mut visit_comment);
    AnnotatedText { segments }
}

pub(super) fn push_segment(segments: &mut Vec<TextSegment>, text: String, offset: usize) {
    if !text.trim().is_empty() {
        segments.push(TextSegment {
            text,
            is_markup: false,
            offset,
        });
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

pub(super) fn strip_triple_slash_doc_comment_with_offset(raw: &str) -> Option<(String, usize)> {
    let mut text_parts = Vec::new();
    let mut first_offset = None;
    let mut running_offset = 0;

    for line in raw.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let Some(stripped) = trimmed.strip_prefix("///") else {
            running_offset += line.len() + 1;
            continue;
        };

        let leading_ws = stripped.len() - stripped.trim_start().len();
        let prose = stripped.trim();
        if !prose.is_empty() {
            first_offset.get_or_insert(running_offset + indent + 3 + leading_ws);
            text_parts.push(prose.to_string());
        }

        running_offset += line.len() + 1;
    }

    let text = text_parts.join(" ");
    first_offset.map(|offset| (text, offset))
}

pub(super) fn strip_doc_block_comment_with_offset(raw: &str) -> Option<(String, usize)> {
    let inner = raw.strip_prefix("/**")?.strip_suffix("*/")?;
    let mut text_parts = Vec::new();
    let mut first_offset = None;
    let mut running_offset = 3;

    for line in inner.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let without_star = trimmed.strip_prefix('*').unwrap_or(trimmed);
        let star_delta = usize::from(trimmed.starts_with('*'));
        let leading_ws = without_star.len() - without_star.trim_start().len();
        let prose = without_star.trim();

        if !prose.is_empty() {
            first_offset.get_or_insert(running_offset + indent + star_delta + leading_ws);
            text_parts.push(prose.to_string());
        }

        running_offset += line.len() + 1;
    }

    let text = text_parts.join(" ").trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some((text, first_offset.unwrap_or(3)))
    }
}

fn is_comment_kind(kind: &str) -> bool {
    matches!(kind, "comment" | "line_comment" | "block_comment")
}
