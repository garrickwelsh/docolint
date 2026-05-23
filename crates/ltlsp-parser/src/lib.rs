use ltlsp_types::{AnnotatedText, TextSegment};

fn language_from_id(id: &str) -> Option<tree_sitter::Language> {
    match id {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "html" => Some(tree_sitter_html::LANGUAGE.into()),
        "json" => Some(tree_sitter_json::LANGUAGE.into()),
        "markdown" | "md" => Some(tree_sitter_md::LANGUAGE.into()),
        "javascript" | "js" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "python" | "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" | "c#" | "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "typescript" | "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    }
}

fn language_from_extension(ext: &str) -> Option<tree_sitter::Language> {
    match ext.trim_start_matches('.') {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "html" | "htm" => Some(tree_sitter_html::LANGUAGE.into()),
        "json" => Some(tree_sitter_json::LANGUAGE.into()),
        "md" | "markdown" => Some(tree_sitter_md::LANGUAGE.into()),
        "js" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    }
}

pub fn parse_document(language_id: &str, content: &str) -> AnnotatedText {
    let lang = language_from_id(language_id)
        .or_else(|| language_from_extension(language_id));

    match lang {
        Some(language) => extract_text(language_id, language, content),
        None => {
            // If it's a known non-prose language ID but we don't have a grammar,
            // or if we are recursing, we might want to return empty.
            // But for the top-level, defaulting to plain text is fine.
            // Actually, let's distinguish between "plain" and "unknown".
            if language_id == "plain" || language_id == "text" || language_id == "unknown" {
                AnnotatedText::from(content)
            } else {
                // For Cycle 8 requirement: unknown fence language -> markup (skip)
                // We'll use a special check in extract_markdown_text instead.
                AnnotatedText::from(content)
            }
        }
    }
}

fn extract_text(language_id: &str, language: tree_sitter::Language, content: &str) -> AnnotatedText {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok();
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return AnnotatedText::from(content),
    };

    match language_id {
        "rust" | "rs" => extract_rust_docs(&tree, content),
        "csharp" | "c#" | "cs" => extract_csharp_docs(&tree, content),
        "html" => extract_html_text(&tree, content),
        "markdown" | "md" => extract_markdown_text(content),
        _ => AnnotatedText::from(content),
    }
}

/// Walk a Rust AST and extract doc comment text as plain segments,
/// everything else as markup.
fn extract_rust_docs(tree: &tree_sitter::Tree, content: &str) -> AnnotatedText {
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    fn walk(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        // line_comment or block_comment that contains outer_doc_comment_marker
        if kind == "line_comment" || kind == "block_comment" {
            // Check for outer_doc_comment_marker child
            let has_doc_marker = (0..node.child_count()).any(|i| {
                node.child(i as u32)
                    .map(|c| c.kind() == "outer_doc_comment_marker")
                    .unwrap_or(false)
            });

            if has_doc_marker {
                // Extract doc_comment child text (the actual prose)
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i as u32) {
                        if child.kind() == "doc_comment" {
                            let start = child.start_byte();
                            let text = std::str::from_utf8(&bytes[start..child.end_byte()])
                                .unwrap_or("")
                                .to_string();
                            if !text.trim().is_empty() {
                                segments.push(TextSegment {
                                    text,
                                    is_markup: false,
                                    offset: start,
                                });
                            }
                        }
                    }
                }
                return;
            }
            // Non-doc comment → markup (skip, no segment added)
            return;
        }

        if cursor.goto_first_child() {
            walk(cursor, bytes, segments);
            while cursor.goto_next_sibling() {
                walk(cursor, bytes, segments);
            }
            cursor.goto_parent();
        }
    }

    walk(&mut cursor, bytes, &mut segments);
    AnnotatedText { segments }
}

/// Walk a C# AST and extract doc comment text as plain segments.
/// C# `comment` nodes contain the raw `/// ...` or `/** ... */` text.
fn extract_csharp_docs(tree: &tree_sitter::Tree, content: &str) -> AnnotatedText {
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    fn walk(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
    ) {
        let node = cursor.node();
        if node.kind() == "comment" {
            let start = node.start_byte();
            let raw = std::str::from_utf8(&bytes[start..node.end_byte()]).unwrap_or("");
            let text = strip_csharp_doc_comment(raw);
            if !text.is_empty() {
                segments.push(TextSegment {
                    text,
                    is_markup: false,
                    offset: start,
                });
            }
            return;
        }

        if cursor.goto_first_child() {
            walk(cursor, bytes, segments);
            while cursor.goto_next_sibling() {
                walk(cursor, bytes, segments);
            }
            cursor.goto_parent();
        }
    }

    walk(&mut cursor, bytes, &mut segments);
    AnnotatedText { segments }
}

/// Strip `///` or `/** */` delimiters from a C# doc comment, returning
/// the plain prose text.
fn strip_csharp_doc_comment(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("///") {
        // Single-line XML doc: strip leading `/// ` or `///`
        trimmed
            .lines()
            .map(|l| l.trim().trim_start_matches("///").trim().to_string())
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string()
    } else if trimmed.starts_with("/**") {
        // Block doc: strip `/**`, `*/`, and leading ` * `
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

/// Walk an HTML AST and extract text nodes, excluding script and style elements.
fn extract_html_text(tree: &tree_sitter::Tree, content: &str) -> AnnotatedText {
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    fn walk(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        if kind == "script_element" || kind == "style_element" {
            // Skip script and style content
            return;
        }

        if kind == "text" {
            let start = node.start_byte();
            let text = std::str::from_utf8(&bytes[start..node.end_byte()])
                .unwrap_or("")
                .to_string();
            if !text.trim().is_empty() {
                segments.push(TextSegment {
                    text,
                    is_markup: false,
                    offset: start,
                });
            }
        }

        if cursor.goto_first_child() {
            walk(cursor, bytes, segments);
            while cursor.goto_next_sibling() {
                walk(cursor, bytes, segments);
            }
            cursor.goto_parent();
        }
    }

    walk(&mut cursor, bytes, &mut segments);
    AnnotatedText { segments }
}

/// Walk a Markdown AST and extract prose from inline nodes and recurse into
/// fenced code blocks.
fn extract_markdown_text(content: &str) -> AnnotatedText {
    let mut parser = tree_sitter_md::MarkdownParser::default();
    let tree = match parser.parse(content.as_bytes(), None) {
        Some(t) => t,
        None => return AnnotatedText::from(content),
    };

    let mut segments = Vec::new();
    let mut cursor = tree.walk();

    fn walk(
        cursor: &mut tree_sitter_md::MarkdownCursor,
        content: &str,
        segments: &mut Vec<TextSegment>,
    ) {
        let node = cursor.node();
        let kind = node.kind();

        // If it's a fenced code block, extract its content and recurse
        if kind == "fenced_code_block" && !cursor.is_inline() {
            let mut lang = "unknown";
            let mut code_content = "";

            let mut child_cursor = node.walk();
            if child_cursor.goto_first_child() {
                loop {
                    let child = child_cursor.node();
                    match child.kind() {
                        "info_string" => {
                            lang = child
                                .utf8_text(content.as_bytes())
                                .unwrap_or("unknown")
                                .trim();
                        }
                        "code_fence_content" => {
                            code_content = child.utf8_text(content.as_bytes()).unwrap_or("");
                        }
                        _ => {}
                    }
                    if !child_cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            if !code_content.is_empty() {
                if let Some(_) =
                    language_from_id(lang).or_else(|| language_from_extension(lang))
                {
                    let content_start = code_fence_node_start(node, content);
                    let mut inner_annotated = parse_document(lang, code_content);
                    for segment in &mut inner_annotated.segments {
                        segment.offset += content_start;
                    }
                    segments.extend(inner_annotated.segments);
                }
            }
            return;
        }

        // Handle gaps and children
        let mut last_offset = node.start_byte();

        // Nodes to skip entirely (and their content)
        let is_markup = matches!(
            kind,
            "emphasis_delimiter"
                | "link_destination"
                | "["
                | "]"
                | "("
                | ")"
                | "atx_h1_marker"
                | "atx_h2_marker"
                | "atx_h3_marker"
                | "atx_h4_marker"
                | "atx_h5_marker"
                | "atx_h6_marker"
                | "fenced_code_block_delimiter"
        );

        if is_markup {
            return;
        }

        if cursor.goto_first_child() {
            loop {
                let child_start = cursor.node().start_byte();
                if child_start > last_offset {
                    let gap_text = &content[last_offset..child_start];
                    if !gap_text.trim().is_empty() {
                        segments.push(TextSegment {
                            text: gap_text.to_string(),
                            is_markup: false,
                            offset: last_offset,
                        });
                    }
                }

                walk(cursor, content, segments);

                last_offset = cursor.node().end_byte();
                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            let node_end = node.end_byte();
            if last_offset < node_end {
                let gap_text = &content[last_offset..node_end];
                if !gap_text.trim().is_empty() {
                    segments.push(TextSegment {
                        text: gap_text.to_string(),
                        is_markup: false,
                        offset: last_offset,
                    });
                }
            }
            cursor.goto_parent();
        } else {
            // Leaf node
            if !is_markup
                && (kind == "inline"
                    || (kind != "paragraph" && kind != "document" && kind != "section"))
            {
                let start = node.start_byte();
                let text = node.utf8_text(content.as_bytes()).unwrap_or("");
                if !text.trim().is_empty() {
                    segments.push(TextSegment {
                        text: text.to_string(),
                        is_markup: false,
                        offset: start,
                    });
                }
            }
        }
    }

    walk(&mut cursor, content, &mut segments);
    AnnotatedText { segments }
}

fn code_fence_node_start(node: tree_sitter::Node, _content: &str) -> usize {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if cursor.node().kind() == "code_fence_content" {
                return cursor.node().start_byte();
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    node.start_byte()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Cycle 5: language mapping ────────────────────────────────────────────

    #[test]
    fn test_language_from_id_rust() {
        assert!(language_from_id("rust").is_some());
    }

    #[test]
    fn test_language_from_id_html() {
        assert!(language_from_id("html").is_some());
    }

    #[test]
    fn test_language_from_id_markdown() {
        assert!(language_from_id("markdown").is_some());
    }

    #[test]
    fn test_language_from_id_json() {
        assert!(language_from_id("json").is_some());
    }

    #[test]
    fn test_language_from_id_csharp() {
        assert!(language_from_id("csharp").is_some());
    }

    #[test]
    fn test_language_from_id_typescript() {
        assert!(language_from_id("typescript").is_some());
    }

    #[test]
    fn test_language_from_id_unknown() {
        assert!(language_from_id("unknown_lang").is_none());
    }

    #[test]
    fn test_language_from_extension_rs() {
        assert!(language_from_extension(".rs").is_some());
    }

    #[test]
    fn test_language_from_extension_md() {
        assert!(language_from_extension("md").is_some());
    }

    #[test]
    fn test_language_from_extension_py() {
        assert!(language_from_extension("py").is_some());
    }

    #[test]
    fn test_language_from_extension_unknown() {
        assert!(language_from_extension(".xyz").is_none());
    }

    #[test]
    fn test_parse_document_unknown_defaults_to_plain() {
        let result = parse_document("unknown", "just some text");
        assert_eq!(result.plain_text(), "just some text");
    }

    // ── Cycle 6: Rust doc comment extraction ────────────────────────────────

    #[test]
    fn test_rust_single_line_doc_comment() {
        let src = "/// Hello world\nfn foo() {}";
        let result = parse_document("rust", src);
        assert_eq!(result.plain_text().trim(), "Hello world");
    }

    #[test]
    fn test_rust_multiple_single_line_doc_comments() {
        let src = "/// First line\n/// Second line\nfn foo() {}";
        let result = parse_document("rust", src);
        let text = result.plain_text();
        assert!(text.contains("First line"), "got: {text}");
        assert!(text.contains("Second line"), "got: {text}");
    }

    #[test]
    fn test_rust_block_doc_comment() {
        let src = "/** Block doc comment */\nfn foo() {}";
        let result = parse_document("rust", src);
        assert_eq!(result.plain_text().trim(), "Block doc comment");
    }

    #[test]
    fn test_rust_non_doc_comment_excluded() {
        let src = "// Regular comment\nfn foo() {}";
        let result = parse_document("rust", src);
        // No doc marker -> no plain text segments
        assert_eq!(result.plain_text(), "");
    }

    #[test]
    fn test_rust_mixed_doc_and_code() {
        let src = "/// Docs here\nfn foo() { let x = 1; }";
        let result = parse_document("rust", src);
        let text = result.plain_text();
        assert!(text.contains("Docs here"), "got: {text}");
        assert!(!text.contains("let"), "code leaked into plain text: {text}");
    }

    // ── Cycle 6: C# doc comment extraction ──────────────────────────────────

    #[test]
    fn test_csharp_single_line_doc_comment() {
        let src = "/// Hello world\npublic void Foo() {}";
        let result = parse_document("csharp", src);
        assert_eq!(result.plain_text(), "Hello world");
    }

    #[test]
    fn test_csharp_multiple_single_line_doc_comments() {
        let src = "/// First line\n/// Second line\npublic void Foo() {}";
        let result = parse_document("csharp", src);
        let text = result.plain_text();
        assert!(text.contains("First line"), "got: {text}");
        assert!(text.contains("Second line"), "got: {text}");
    }

    #[test]
    fn test_csharp_block_doc_comment() {
        let src = "/** Block doc comment */\npublic void Foo() {}";
        let result = parse_document("csharp", src);
        assert_eq!(result.plain_text(), "Block doc comment");
    }

    #[test]
    fn test_csharp_non_doc_comment_excluded() {
        // Single-slash comment is not a doc comment in C#
        let src = "// Regular comment\npublic void Foo() {}";
        let result = parse_document("csharp", src);
        assert_eq!(result.plain_text(), "");
    }

    #[test]
    fn test_csharp_xml_doc_text_extracted() {
        let src = "/// <summary>Does something useful</summary>\npublic void Foo() {}";
        let result = parse_document("csharp", src);
        let text = result.plain_text();
        assert!(text.contains("Does something useful"), "got: {text}");
    }

    #[test]
    fn test_csharp_mixed_doc_and_code() {
        let src = "/// Docs here\npublic void Foo() { int x = 1; }";
        let result = parse_document("csharp", src);
        let text = result.plain_text();
        assert!(text.contains("Docs here"), "got: {text}");
        assert!(!text.contains("int"), "code leaked into plain text: {text}");
    }

    // ── Cycle 7: HTML text extraction ───────────────────────────────────────

    #[test]
    fn test_html_text_extraction() {
        let src = "<div><p>Hello world</p></div>";
        let result = parse_document("html", src);
        assert_eq!(result.plain_text(), "Hello world");
    }

    #[test]
    fn test_html_multiple_tags() {
        let src = "<ul><li>One</li><li>Two</li></ul>";
        let result = parse_document("html", src);
        let text = result.plain_text();
        assert!(text.contains("One"));
        assert!(text.contains("Two"));
    }

    #[test]
    fn test_html_script_exclusion() {
        let src = "<div><p>Visible</p><script>console.log('hidden')</script></div>";
        let result = parse_document("html", src);
        let text = result.plain_text();
        assert!(text.contains("Visible"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("hidden"));
    }

    #[test]
    fn test_html_style_exclusion() {
        let src = "<div><p>Visible</p><style>.hidden { display: none; }</style></div>";
        let result = parse_document("html", src);
        let text = result.plain_text();
        assert!(text.contains("Visible"));
        assert!(!text.contains(".hidden"));
    }

    // ── Cycle 8: Markdown recursive parsing ─────────────────────────────────

    #[test]
    fn test_markdown_prose_extraction() {
        let src = "Hello *world*";
        let result = parse_document("markdown", src);
        let text = result.plain_text();
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(!text.contains("*"));
    }

    #[test]
    fn test_markdown_fenced_code_rust_recursion() {
        let src = "# Title\n\n```rust\n/// Doc comment\nfn foo() {}\n```";
        let result = parse_document("markdown", src);
        let text = result.plain_text();
        assert!(text.contains("Title"));
        assert!(text.contains("Doc comment"));
        assert!(!text.contains("fn foo"));
    }

    #[test]
    fn test_markdown_unknown_fence_ignored() {
        let src = "```unknown\nCheck me not\n```";
        let result = parse_document("markdown", src);
        let text = result.plain_text();
        assert!(!text.contains("Check me not"));
    }

    // ── Cycle 9: Absolute byte offset tracking ──────────────────────────────

    #[test]
    fn test_rust_offset_tracking() {
        let src = "fn main() {} \n/// Doc comment";
        let result = parse_document("rust", src);
        assert_eq!(result.segments.len(), 1);
        let start = src.find(" Doc comment").unwrap();
        assert_eq!(result.segments[0].offset, start);
    }

    #[test]
    fn test_markdown_recursive_offset_tracking() {
        let src = "# Title\n\n```rust\n/// Doc\n```";
        let result = parse_document("markdown", src);
        // Find segment for "Doc"
        let doc_seg = result.segments.iter().find(|s| s.text.contains("Doc")).unwrap();
        let expected_offset = src.find(" Doc").unwrap();
        assert_eq!(doc_seg.offset, expected_offset);
    }
}
