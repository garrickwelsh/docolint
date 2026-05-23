use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use ltlsp_types::GrammarError;

pub struct Dictionary {
    ignored_words: HashSet<String>,
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl Dictionary {
    pub fn new() -> Self {
        Self {
            ignored_words: HashSet::new(),
        }
    }

    pub fn load(workspace_root: &Path, document_path: &Path) -> Self {
        let mut ignored_words = HashSet::new();
        
        let mut current = if document_path.is_file() {
            document_path.parent()
        } else {
            Some(document_path)
        };

        while let Some(path) = current {
            let ignore_file = path.join(".ltlsp-ignore");
            if let Ok(content) = fs::read_to_string(ignore_file) {
                for line in content.lines() {
                    let word = line.trim();
                    if !word.is_empty() && !word.starts_with('#') {
                        ignored_words.insert(word.to_lowercase());
                    }
                }
            }

            if path == workspace_root {
                break;
            }
            current = path.parent();
        }

        Self { ignored_words }
    }

    pub fn is_ignored(&self, word: &str) -> bool {
        self.ignored_words.contains(&word.to_lowercase())
    }

    pub fn add_word(&mut self, word: &str, target_file: &Path) -> std::io::Result<()> {
        let word = word.trim().to_lowercase();
        if word.is_empty() {
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(target_file)?;

        writeln!(file, "{}", word)?;
        self.ignored_words.insert(word);
        Ok(())
    }

    pub fn filter_errors(&self, text: &str, errors: Vec<GrammarError>) -> Vec<GrammarError> {
        errors.into_iter().filter(|error| {
            if error.offset + error.length > text.len() {
                return true;
            }
            let word = &text[error.offset..(error.offset + error.length)];
            !self.is_ignored(word)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_and_merge_ignores() {
        let root = tempdir().unwrap();
        let root_path = root.path();
        let sub = root_path.join("sub");
        fs::create_dir(&sub).unwrap();
        
        let mut root_ignore = File::create(root_path.join(".ltlsp-ignore")).unwrap();
        writeln!(root_ignore, "rootword").unwrap();
        
        let mut sub_ignore = File::create(sub.join(".ltlsp-ignore")).unwrap();
        writeln!(sub_ignore, "subword").unwrap();
        
        let dict = Dictionary::load(root_path, &sub.join("file.rs"));
        
        assert!(dict.is_ignored("rootword"));
        assert!(dict.is_ignored("subword"));
        assert!(!dict.is_ignored("unknown"));
    }

    #[test]
    fn test_is_ignored_case_insensitive() {
        let mut dict = Dictionary::new();
        dict.ignored_words.insert("word".to_string());
        
        assert!(dict.is_ignored("word"));
        assert!(dict.is_ignored("WORD"));
    }

    #[test]
    fn test_add_word_creates_file() {
        let root = tempdir().unwrap();
        let root_path = root.path();
        let ignore_file = root_path.join(".ltlsp-ignore");
        
        let mut dict = Dictionary::new();
        dict.add_word("newword", &ignore_file).unwrap();
        
        assert!(ignore_file.exists());
        let content = fs::read_to_string(ignore_file).unwrap();
        assert!(content.contains("newword"));
        assert!(dict.is_ignored("newword"));
    }

    #[test]
    fn test_filter_errors() {
        let mut dict = Dictionary::new();
        dict.ignored_words.insert("ignored".to_string());
        
        let text = "This has an ignored word and a valid word.";
        let errors = vec![
            GrammarError {
                message: "Error 1".to_string(),
                offset: 12,
                length: 7, // "ignored"
                replacements: vec![],
                rule_id: "RULE1".to_string(),
            },
            GrammarError {
                message: "Error 2".to_string(),
                offset: 31,
                length: 5, // "valid"
                replacements: vec![],
                rule_id: "RULE2".to_string(),
            },
        ];
        
        let filtered = dict.filter_errors(text, errors);
        
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rule_id, "RULE2");
    }
}
