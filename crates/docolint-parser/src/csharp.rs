use crate::ParserConfig;
use crate::comments::{extract_comment_segments, push_segment, strip_inline_comment_with_offset};
use docolint_types::AnnotatedText;

/// Walk a C# AST and extract doc comment text as plain segments.
/// C# `comment` nodes contain the raw `/// ...` or `/** ... */` text.
pub(super) fn extract_csharp_docs(
    tree: &tree_sitter::Tree,
    content: &str,
    config: &ParserConfig,
) -> AnnotatedText {
    extract_comment_segments(tree, content, |node, raw, segments| {
        let start = node.start_byte();
        if let Some((text, offset_delta)) =
            extract_csharp_comment(raw, config.include_inline_comments)
        {
            push_segment(segments, text, start + offset_delta);
        }
    })
}

fn extract_csharp_comment(raw: &str, include_inline: bool) -> Option<(String, usize)> {
    let trimmed = raw.trim();
    if trimmed.starts_with("///") || trimmed.starts_with("/**") {
        let text = strip_csharp_doc_comment(raw);
        if text.is_empty() {
            None
        } else {
            Some((text, 0))
        }
    } else if include_inline {
        strip_inline_comment_with_offset(raw)
    } else {
        None
    }
}

/// Strip `///` or `/** */` delimiters from a C# doc comment, returning
/// the plain prose text.
fn strip_csharp_doc_comment(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("///") {
        trimmed
            .lines()
            .map(|l| l.trim().trim_start_matches("///").trim().to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else if trimmed.starts_with("/**") {
        trimmed
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim().to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else {
        String::new()
    }
}
