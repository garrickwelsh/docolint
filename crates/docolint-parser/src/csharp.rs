use crate::ParserConfig;
use crate::comments::{
    extract_comment_segments, push_segment, strip_doc_block_comment_with_offset,
    strip_inline_comment_with_offset, strip_triple_slash_doc_comment_with_offset,
};
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
    if trimmed.starts_with("///") {
        strip_triple_slash_doc_comment_with_offset(raw)
    } else if trimmed.starts_with("/**") {
        strip_doc_block_comment_with_offset(raw)
    } else if include_inline {
        strip_inline_comment_with_offset(raw)
    } else {
        None
    }
}
