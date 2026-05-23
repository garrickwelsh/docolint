use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct GrammarError {
    pub message: String,
    pub offset: usize,
    pub length: usize,
    pub replacements: Vec<String>,
    pub rule_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextSegment {
    pub text: String,
    #[serde(rename = "markup")]
    pub is_markup: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotatedText {
    pub segments: Vec<TextSegment>,
}

impl From<&str> for AnnotatedText {
    fn from(text: &str) -> Self {
        AnnotatedText {
            segments: vec![TextSegment {
                text: text.to_string(),
                is_markup: false,
            }],
        }
    }
}

impl AnnotatedText {
    pub fn plain_text(&self) -> String {
        self.segments
            .iter()
            .filter(|s| !s.is_markup)
            .map(|s| s.text.as_str())
            .collect()
    }
}
