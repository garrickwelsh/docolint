use docolint_types::{AnnotatedText, TextSegment};

/// Configuration for document parsing behavior.
#[derive(Debug, Clone, Default)]
pub struct ParserConfig {
    /// When `true`, inline comments (`//`, `/* */` non-doc) are extracted as prose
    /// for languages that distinguish doc comments from inline comments.
    /// Has no effect on languages without doc comment conventions (Bash, Python, etc.).
    pub include_inline_comments: bool,
}

fn language_from_id(id: &str) -> Option<tree_sitter::Language> {
    match id {
        "rust" => Some(tree_sitter_rust::LANGUAGE.into()),
        "html" => Some(tree_sitter_html::LANGUAGE.into()),
        "markdown" | "md" => Some(tree_sitter_md::LANGUAGE.into()),
        "javascript" | "js" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "python" | "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "csharp" | "c#" | "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "typescript" | "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "css" => Some(tree_sitter_css::LANGUAGE.into()),
        "lua" => Some(tree_sitter_lua::LANGUAGE.into()),
        "bash" | "sh" | "zsh" => Some(tree_sitter_bash::LANGUAGE.into()),
        "powershell" | "pwsh" => Some(tree_sitter_powershell::LANGUAGE.into()),
        "scss" => Some(tree_sitter_scss::language()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        _ => None,
    }
}

fn language_from_extension(ext: &str) -> Option<tree_sitter::Language> {
    match ext.trim_start_matches('.') {
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "html" | "htm" => Some(tree_sitter_html::LANGUAGE.into()),
        "md" | "markdown" => Some(tree_sitter_md::LANGUAGE.into()),
        "js" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        "ts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "css" => Some(tree_sitter_css::LANGUAGE.into()),
        "lua" => Some(tree_sitter_lua::LANGUAGE.into()),
        "sh" | "bash" | "zsh" => Some(tree_sitter_bash::LANGUAGE.into()),
        "ps1" | "psm1" | "pwsh" => Some(tree_sitter_powershell::LANGUAGE.into()),
        "scss" => Some(tree_sitter_scss::language()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        _ => None,
    }
}

/// Parses source content and extracts prose segments for grammar checking.
///
/// Uses `tree-sitter` to identify doc comments, HTML text nodes, and markdown prose.
/// Code, markup delimiters, and non-prose content are marked as `is_markup: true`
/// so LanguageTool skips them during checking.
///
/// # Arguments
/// * `language_id` - LSP language identifier (e.g., `"rust"`, `"markdown"`, `"html"`).
///   Also accepts file extensions (e.g., `"rs"`, `"md"`, `"py"`). Falls back to
///   plain text for unknown languages.
/// * `content` - The full source file content to parse.
///
/// # Returns
/// An [`AnnotatedText`] containing ordered text segments with byte offsets mapped
/// back to the original content.
pub fn parse_document(language_id: &str, content: &str, config: &ParserConfig) -> AnnotatedText {
    let lang = language_from_id(language_id)
        .or_else(|| language_from_extension(language_id));

    match lang {
        Some(language) => extract_text(language_id, language, content, config),
        None => AnnotatedText::from(content),
    }
}

fn extract_text(language_id: &str, language: tree_sitter::Language, content: &str, config: &ParserConfig) -> AnnotatedText {
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
        "markdown" | "md" => extract_markdown_text(content, config),
        "css" => extract_comment_docs(&tree, content, language_id, config),
        "lua" => extract_comment_docs(&tree, content, language_id, config),
        "bash" | "sh" | "zsh" => extract_comment_docs(&tree, content, language_id, config),
        "powershell" | "pwsh" => extract_comment_docs(&tree, content, language_id, config),
        "scss" => extract_comment_docs(&tree, content, language_id, config),
        "python" | "py" => extract_comment_docs(&tree, content, language_id, config),
        "java" => extract_comment_docs(&tree, content, language_id, config),
        "javascript" | "js" => extract_comment_docs(&tree, content, language_id, config),
        "typescript" | "ts" => extract_comment_docs(&tree, content, language_id, config),
        "tsx" => extract_comment_docs(&tree, content, language_id, config),
        _ => AnnotatedText::from(content),
    }
}

/// Walk a tree-sitter AST and extract comment text as prose segments.
/// Strips comment delimiters before returning text.
///
/// For languages with doc comment conventions (JS/TS/Java):
///   - `/** */` always extracted
///   - `//` and `/* non-doc */` only if config.include_inline_comments
///
/// For languages without doc conventions, all comments are extracted.
fn extract_comment_docs(tree: &tree_sitter::Tree, content: &str, language_id: &str, config: &ParserConfig) -> AnnotatedText {
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut cursor = tree.walk();
    let bytes = content.as_bytes();

    let has_doc_comments = matches!(language_id,
        "javascript" | "js" | "typescript" | "ts" | "tsx" | "java"
    );

    fn walk(
        cursor: &mut tree_sitter::TreeCursor,
        bytes: &[u8],
        segments: &mut Vec<TextSegment>,
        has_doc_comments: bool,
        include_inline: bool,
    ) {
        let node = cursor.node();
        let kind = node.kind();
        if kind == "comment" || kind == "block_comment" || kind == "line_comment" {
            let start = node.start_byte();
            let raw = std::str::from_utf8(&bytes[start..node.end_byte()]).unwrap_or("");
            if let Some(text) = strip_comment_delimiters(raw, has_doc_comments, include_inline)
                && !text.is_empty()
            {
                segments.push(TextSegment {
                    text,
                    is_markup: false,
                    offset: start,
                });
            }
            return;
        }

        if cursor.goto_first_child() {
            walk(cursor, bytes, segments, has_doc_comments, include_inline);
            while cursor.goto_next_sibling() {
                walk(cursor, bytes, segments, has_doc_comments, include_inline);
            }
            cursor.goto_parent();
        }
    }

    walk(&mut cursor, bytes, &mut segments, has_doc_comments, config.include_inline_comments);
    AnnotatedText { segments }
}

/// Strip comment delimiters from raw comment text.
/// Returns None if the comment should be skipped.
///
/// For doc-comment languages: returns None for inline comments when include_inline is false.
fn strip_comment_delimiters(raw: &str, has_doc_comments: bool, include_inline: bool) -> Option<String> {
    let trimmed = raw.trim();

    if has_doc_comments {
        if trimmed.starts_with("/**") {
            let inner = trimmed
                .trim_start_matches("/**")
                .trim_end_matches("*/")
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim().to_string())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            if inner.is_empty() { None } else { Some(inner) }
        } else if trimmed.starts_with("//") {
            if !include_inline { return None; }
            let text = trimmed.trim_start_matches("//").trim().to_string();
            if text.is_empty() { None } else { Some(text) }
        } else {
            if !include_inline { return None; }
            let text = trimmed.trim_start_matches("/*").trim_end_matches("*/").trim().to_string();
            if text.is_empty() { None } else { Some(text) }
        }
    } else if trimmed.starts_with("<#") && trimmed.ends_with("#>") {
        let text = trimmed.trim_start_matches("<#").trim_end_matches("#>").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else if trimmed.starts_with("--[[") && trimmed.ends_with("--]]") {
        let text = trimmed.trim_start_matches("--[[").trim_end_matches("--]]").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else if trimmed.starts_with("--") {
        let text = trimmed.trim_start_matches("--").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
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
        if inner.is_empty() { None } else { Some(inner) }
    } else if trimmed.starts_with("//") {
        let text = trimmed.trim_start_matches("//").trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else if trimmed.starts_with('#') {
        let text = trimmed.trim_start_matches('#').trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    } else {
        Some(trimmed.to_string())
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
                    if let Some(child) = node.child(i as u32).filter(|c| c.kind() == "doc_comment") {
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
fn extract_markdown_text(content: &str, config: &ParserConfig) -> AnnotatedText {
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
        config: &ParserConfig,
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

                if !code_content.is_empty()
                    && (language_from_id(lang).is_some() || language_from_extension(lang).is_some())
                {
                    let content_start = code_fence_node_start(node, content);
                    let mut inner_annotated = parse_document(lang, code_content, config);
                    for segment in &mut inner_annotated.segments {
                        segment.offset += content_start;
                    }
                    segments.extend(inner_annotated.segments);
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

                walk(cursor, content, segments, config);

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

    walk(&mut cursor, content, &mut segments, config);
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
        assert!(language_from_id("json").is_none());
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
        let result = parse_document("unknown", "just some text", &ParserConfig::default());
        assert_eq!(result.plain_text(), "just some text");
    }

    // ── Cycle 6: Rust doc comment extraction ────────────────────────────────

    #[test]
    fn test_rust_single_line_doc_comment() {
        let src = "/// Hello world\nfn foo() {}";
        let result = parse_document("rust", src, &ParserConfig::default());
        assert_eq!(result.plain_text().trim(), "Hello world");
    }

    #[test]
    fn test_rust_multiple_single_line_doc_comments() {
        let src = "/// First line\n/// Second line\nfn foo() {}";
        let result = parse_document("rust", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("First line"), "got: {text}");
        assert!(text.contains("Second line"), "got: {text}");
    }

    #[test]
    fn test_rust_block_doc_comment() {
        let src = "/** Block doc comment */\nfn foo() {}";
        let result = parse_document("rust", src, &ParserConfig::default());
        assert_eq!(result.plain_text().trim(), "Block doc comment");
    }

    #[test]
    fn test_rust_non_doc_comment_excluded() {
        let src = "// Regular comment\nfn foo() {}";
        let result = parse_document("rust", src, &ParserConfig::default());
        // No doc marker -> no plain text segments
        assert_eq!(result.plain_text(), "");
    }

    #[test]
    fn test_rust_mixed_doc_and_code() {
        let src = "/// Docs here\nfn foo() { let x = 1; }";
        let result = parse_document("rust", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Docs here"), "got: {text}");
        assert!(!text.contains("let"), "code leaked into plain text: {text}");
    }

    // ── Cycle 6: C# doc comment extraction ──────────────────────────────────

    #[test]
    fn test_csharp_single_line_doc_comment() {
        let src = "/// Hello world\npublic void Foo() {}";
        let result = parse_document("csharp", src, &ParserConfig::default());
        assert_eq!(result.plain_text(), "Hello world");
    }

    #[test]
    fn test_csharp_multiple_single_line_doc_comments() {
        let src = "/// First line\n/// Second line\npublic void Foo() {}";
        let result = parse_document("csharp", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("First line"), "got: {text}");
        assert!(text.contains("Second line"), "got: {text}");
    }

    #[test]
    fn test_csharp_block_doc_comment() {
        let src = "/** Block doc comment */\npublic void Foo() {}";
        let result = parse_document("csharp", src, &ParserConfig::default());
        assert_eq!(result.plain_text(), "Block doc comment");
    }

    #[test]
    fn test_csharp_non_doc_comment_excluded() {
        // Single-slash comment is not a doc comment in C#
        let src = "// Regular comment\npublic void Foo() {}";
        let result = parse_document("csharp", src, &ParserConfig::default());
        assert_eq!(result.plain_text(), "");
    }

    #[test]
    fn test_csharp_xml_doc_text_extracted() {
        let src = "/// <summary>Does something useful</summary>\npublic void Foo() {}";
        let result = parse_document("csharp", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Does something useful"), "got: {text}");
    }

    #[test]
    fn test_csharp_mixed_doc_and_code() {
        let src = "/// Docs here\npublic void Foo() { int x = 1; }";
        let result = parse_document("csharp", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Docs here"), "got: {text}");
        assert!(!text.contains("int"), "code leaked into plain text: {text}");
    }

    // ── Cycle 7: HTML text extraction ───────────────────────────────────────

    #[test]
    fn test_html_text_extraction() {
        let src = "<div><p>Hello world</p></div>";
        let result = parse_document("html", src, &ParserConfig::default());
        assert_eq!(result.plain_text(), "Hello world");
    }

    #[test]
    fn test_html_multiple_tags() {
        let src = "<ul><li>One</li><li>Two</li></ul>";
        let result = parse_document("html", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("One"));
        assert!(text.contains("Two"));
    }

    #[test]
    fn test_html_script_exclusion() {
        let src = "<div><p>Visible</p><script>console.log('hidden')</script></div>";
        let result = parse_document("html", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Visible"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("hidden"));
    }

    #[test]
    fn test_html_style_exclusion() {
        let src = "<div><p>Visible</p><style>.hidden { display: none; }</style></div>";
        let result = parse_document("html", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Visible"));
        assert!(!text.contains(".hidden"));
    }

    // ── Cycle 8: Markdown recursive parsing ─────────────────────────────────

    #[test]
    fn test_markdown_prose_extraction() {
        let src = "Hello *world*";
        let result = parse_document("markdown", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(!text.contains("*"));
    }

    #[test]
    fn test_markdown_fenced_code_rust_recursion() {
        let src = "# Title\n\n```rust\n/// Doc comment\nfn foo() {}\n```";
        let result = parse_document("markdown", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Title"));
        assert!(text.contains("Doc comment"));
        assert!(!text.contains("fn foo"));
    }

    #[test]
    fn test_markdown_unknown_fence_ignored() {
        let src = "```unknown\nCheck me not\n```";
        let result = parse_document("markdown", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(!text.contains("Check me not"));
    }

    #[test]
    fn test_markdown_empty_fence_ignored() {
        let src = "Hello world\n\n```\nCheck me not\n```\n\nMore prose here";
        let result = parse_document("markdown", src, &ParserConfig::default());
        let text = result.plain_text();
        assert!(text.contains("Hello world"));
        assert!(text.contains("More prose here"));
        assert!(!text.contains("Check me not"));
    }

    // ── Cycle 9: Absolute byte offset tracking ──────────────────────────────

    #[test]
    fn test_rust_offset_tracking() {
        let src = "fn main() {} \n/// Doc comment";
        let result = parse_document("rust", src, &ParserConfig::default());
        assert_eq!(result.segments.len(), 1);
        let start = src.find(" Doc comment").unwrap();
        assert_eq!(result.segments[0].offset, start);
    }

    #[test]
    fn test_markdown_recursive_offset_tracking() {
        let src = "# Title\n\n```rust\n/// Doc\n```";
        let result = parse_document("markdown", src, &ParserConfig::default());
        let doc_seg = result.segments.iter().find(|s| s.text.contains("Doc")).unwrap();
        let expected_offset = src.find(" Doc").unwrap();
        assert_eq!(doc_seg.offset, expected_offset);
    }

    // ── Cycle 1: CSS comment extraction (tracer bullet) ──────────────────────

    #[test]
    fn test_css_comment_extraction() {
        let config = ParserConfig::default();
        let src = ".foo { /* Hello world */ }";
        let result = parse_document("css", src, &config);
        assert_eq!(result.plain_text().trim(), "Hello world");
    }

    // ── Cycle 2: Lua comment extraction ──────────────────────────────────────

    #[test]
    fn test_lua_comment_extraction() {
        let config = ParserConfig::default();
        let src = "-- Line comment\nlocal x = 1\n--[[ Block comment ]]";
        let result = parse_document("lua", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Line comment"));
        assert!(text.contains("Block comment"));
    }

    // ── Cycle 3: Bash comment extraction ─────────────────────────────────────

    #[test]
    fn test_bash_comment_extraction() {
        let config = ParserConfig::default();
        let src = "# Hello world\necho hi";
        let result = parse_document("bash", src, &config);
        assert_eq!(result.plain_text().trim(), "Hello world");
    }

    // ── Cycle 4: PowerShell comment extraction ───────────────────────────────

    #[test]
    fn test_powershell_comment_extraction() {
        let config = ParserConfig::default();
        let src = "# Line comment\n<# Block comment #>\nWrite-Host hi";
        let result = parse_document("powershell", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Line comment"));
        assert!(text.contains("Block comment"));
    }

    // ── Cycle 5: SCSS comment extraction ─────────────────────────────────────

    #[test]
    fn test_scss_comment_extraction() {
        let config = ParserConfig::default();
        let src = "/* Block comment */\n.foo { color: red; }";
        let result = parse_document("scss", src, &config);
        assert_eq!(result.plain_text().trim(), "Block comment");
    }

    // ── Cycle 6: Python comment extraction ───────────────────────────────────

    #[test]
    fn test_python_comment_extraction() {
        let config = ParserConfig::default();
        let src = "# Hello world\nx = 1";
        let result = parse_document("python", src, &config);
        assert_eq!(result.plain_text().trim(), "Hello world");
    }

    // ── Cycle 7: Java doc vs inline distinction ──────────────────────────────

    #[test]
    fn test_java_doc_only() {
        let config = ParserConfig::default();
        let src = "/** Doc comment */\n// Inline comment\nclass Foo {}";
        let result = parse_document("java", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Doc comment"));
        assert!(!text.contains("Inline comment"));
    }

    // ── Cycle 8: Java inline comments when enabled ───────────────────────────

    #[test]
    fn test_java_with_inline_comments() {
        let config = ParserConfig { include_inline_comments: true };
        let src = "/** Doc */\n// Inline\nclass Foo {}";
        let result = parse_document("java", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Doc"));
        assert!(text.contains("Inline"));
    }

    // ── Cycle 9: JavaScript doc vs inline ────────────────────────────────────

    #[test]
    fn test_js_doc_only() {
        let config = ParserConfig::default();
        let src = "/** Doc */\n// Inline\nconst x = 1;";
        let result = parse_document("javascript", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Doc"));
        assert!(!text.contains("Inline"));
    }

    // ── Cycle 11: Markdown recursion with new language ───────────────────────

    #[test]
    fn test_markdown_java_recursion() {
        let config = ParserConfig::default();
        let src = "# Title\n\n```java\n/** Doc in Java */\n```";
        let result = parse_document("markdown", src, &config);
        let text = result.plain_text();
        assert!(text.contains("Title"));
        assert!(text.contains("Doc in Java"));
    }
}
