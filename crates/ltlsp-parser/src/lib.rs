use ltlsp_types::AnnotatedText;

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
        Some(language) => parse_with_language(language, content),
        None => AnnotatedText::from(content),
    }
}

fn parse_with_language(language: tree_sitter::Language, content: &str) -> AnnotatedText {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok();
    let _tree = parser.parse(content, None);
    // Placeholder: full extraction implemented in Cycles 6-8
    AnnotatedText::from(content)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_parse_document_rust_no_panic() {
        let result = parse_document("rust", "fn main() {}");
        assert!(!result.segments.is_empty());
    }

    #[test]
    fn test_parse_document_unknown_defaults_to_plain() {
        let result = parse_document("unknown", "just some text");
        assert_eq!(result.plain_text(), "just some text");
    }
}
