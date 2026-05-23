use serde::Serialize;

/// Represents a single grammar or spelling error returned by LanguageTool.
///
/// This struct maps directly from the LanguageTool API response and is used
/// internally to track error location, message, and suggested replacements.
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarError {
    /// Human-readable description of the error.
    pub message: String,
    /// Byte offset of the error within the plain text (excluding markup segments).
    pub offset: usize,
    /// Length of the problematic text in bytes.
    pub length: usize,
    /// Suggested replacement strings, ordered by preference.
    pub replacements: Vec<String>,
    /// LanguageTool rule identifier that triggered this error.
    pub rule_id: String,
}

/// A segment of text extracted from source code, with metadata for LanguageTool processing.
///
/// Segments are either plain prose (checked by LanguageTool) or markup (skipped during
/// checking but preserved for offset mapping). The `offset` field tracks the segment's
/// position in the original source file.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextSegment {
    /// The text content of this segment.
    pub text: String,
    /// When `true`, LanguageTool ignores this segment during checking.
    /// Used for code, HTML tags, markdown delimiters, etc.
    ///
    /// Serialized as `"markup"` for LanguageTool API compatibility.
    #[serde(rename = "markup")]
    pub is_markup: bool,
    /// Byte offset of this segment in the original source content.
    ///
    /// Skipped during serialization (`#[serde(skip)]`) as it is internal-only.
    #[serde(skip)]
    pub offset: usize,
}

/// A collection of [`TextSegment`]s representing extracted prose from a source file.
///
/// This is the primary output of the parser crate. It separates human-readable text
/// from code/markup, enabling LanguageTool to check only the relevant portions while
/// maintaining accurate byte offset mappings back to the original file.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotatedText {
    /// Ordered segments of text extracted from the source.
    pub segments: Vec<TextSegment>,
}

impl From<&str> for AnnotatedText {
    fn from(text: &str) -> Self {
        AnnotatedText {
            segments: vec![TextSegment {
                text: text.to_string(),
                is_markup: false,
                offset: 0,
            }],
        }
    }
}

impl AnnotatedText {
    /// Returns all non-markup segment text concatenated.
    ///
    /// Use this to get the plain text string that LanguageTool actually checks.
    /// Offsets returned by LanguageTool are relative to this string.
    pub fn plain_text(&self) -> String {
        self.segments
            .iter()
            .filter(|s| !s.is_markup)
            .map(|s| s.text.as_str())
            .collect()
    }
}
