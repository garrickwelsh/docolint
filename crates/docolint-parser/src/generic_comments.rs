use crate::ParserConfig;
use crate::comments::{
    extract_comment_segments, push_retained_comment_lines, push_segment,
    retained_doc_block_comment_lines, retained_plain_block_comment_lines,
    strip_doc_block_comment_with_offset,
};
use docolint_types::AnnotatedText;

/// Walk a tree-sitter AST and extract comment text as prose segments.
/// Strips comment delimiters before returning text.
///
/// For languages with doc comment conventions (JS/TS/Java):
///   - `/** */` always extracted
///   - `//` and `/* non-doc */` only if config.include_inline_comments
///
/// For languages without doc conventions, all comments are extracted.
pub(super) fn extract_comment_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    language_id: &str,
    config: &ParserConfig,
    next_unit_id: &mut usize,
) -> AnnotatedText {
    let has_doc_comments = matches!(
        language_id,
        "javascript" | "js" | "typescript" | "ts" | "tsx" | "java"
    );

    extract_comment_segments(
        tree,
        content,
        next_unit_id,
        |node, raw, segments, unit_id| {
            if raw.contains('\n') {
                if has_doc_comments && raw.trim_start().starts_with("/**") {
                    if let Some(lines) = retained_doc_block_comment_lines(raw) {
                        push_retained_comment_lines(segments, node.start_byte(), lines, unit_id);
                        return;
                    }
                } else if !has_doc_comments && raw.trim_start().starts_with("/*") {
                    if let Some(lines) = retained_plain_block_comment_lines(raw) {
                        push_retained_comment_lines(segments, node.start_byte(), lines, unit_id);
                        return;
                    }
                }
            }

            if let Some((text, offset_delta)) =
                strip_comment_delimiters(raw, has_doc_comments, config.include_inline_comments)
            {
                push_segment(segments, text, node.start_byte() + offset_delta, unit_id);
            }
        },
    )
}

/// Strip comment delimiters from raw comment text.
/// Returns None if the comment should be skipped.
///
/// For doc-comment languages: returns None for inline comments when include_inline is false.
fn strip_comment_delimiters(
    raw: &str,
    has_doc_comments: bool,
    include_inline: bool,
) -> Option<(String, usize)> {
    let trimmed = raw.trim();

    if has_doc_comments {
        if trimmed.starts_with("/**") {
            strip_doc_block_comment_with_offset(raw)
        } else if trimmed.starts_with("//") {
            if !include_inline {
                return None;
            }
            let text = trimmed.trim_start_matches("//").trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some((text, 0))
            }
        } else {
            if !include_inline {
                return None;
            }
            let text = trimmed
                .trim_start_matches("/*")
                .trim_end_matches("*/")
                .trim()
                .to_string();
            if text.is_empty() {
                None
            } else {
                Some((text, 0))
            }
        }
    } else if trimmed.starts_with("<#") && trimmed.ends_with("#>") {
        let text = trimmed
            .trim_start_matches("<#")
            .trim_end_matches("#>")
            .trim()
            .to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else if trimmed.starts_with("--[[") && trimmed.ends_with("--]]") {
        let text = trimmed
            .trim_start_matches("--[[")
            .trim_end_matches("--]]")
            .trim()
            .to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else if trimmed.starts_with("--") {
        let text = trimmed.trim_start_matches("--").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
        let inner = trimmed
            .trim_start_matches("/*")
            .trim_end_matches("*/")
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim().to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if inner.is_empty() {
            None
        } else {
            Some((inner, 0))
        }
    } else if trimmed.starts_with("//") {
        let text = trimmed.trim_start_matches("//").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else if trimmed.starts_with('#') {
        let text = trimmed.trim_start_matches('#').trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else {
        Some((trimmed.to_string(), 0))
    }
}
