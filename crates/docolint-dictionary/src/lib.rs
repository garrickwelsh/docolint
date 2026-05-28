use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use docolint_types::GrammarError;

/// Manages a set of ignored words for filtering grammar errors.
///
/// Loads `.docolint-ignore` files hierarchically from the document's directory up to
/// the workspace root. Words are stored case-insensitively (lowercased). Supports
/// adding new words to a target ignore file and filtering errors based on ignored words.
pub struct Dictionary {
    ignored_words: HashSet<String>,
}

impl Default for Dictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl Dictionary {
    fn char_offset_to_byte_offset(text: &str, char_offset: usize) -> Option<usize> {
        if char_offset == text.chars().count() {
            return Some(text.len());
        }

        text.char_indices().nth(char_offset).map(|(idx, _)| idx)
    }

    /// Creates an empty dictionary with no ignored words.
    pub fn new() -> Self {
        Self {
            ignored_words: HashSet::new(),
        }
    }

    /// Loads and merges `.docolint-ignore` files from `document_path` up to `workspace_root`.
    ///
    /// Walks the directory tree upward, reading each `.docolint-ignore` file found.
    /// Lines starting with `#` are treated as comments and skipped. Empty lines are ignored.
    /// Words are lowercased before storage.
    ///
    /// # Arguments
    /// * `workspace_root` - The root directory to stop walking at. Must be an ancestor
    ///   of (or equal to) `document_path`'s parent.
    /// * `document_path` - Path to the source file being checked. If this is a file,
    ///   its parent directory is used as the starting point.
    ///
    /// # Panics
    /// Does not panic. File read errors are silently ignored (missing files = no words).
    pub fn load(workspace_root: &Path, document_path: &Path) -> Self {
        let mut ignored_words = HashSet::new();
        
        let mut current = if document_path.is_file() {
            document_path.parent()
        } else {
            Some(document_path)
        };

        while let Some(path) = current {
            let ignore_file = path.join(".docolint-ignore");
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

    /// Checks if a word is in the ignored set (case-insensitive).
    ///
    /// # Arguments
    /// * `word` - The word to check. Compared in lowercase against stored words.
    pub fn is_ignored(&self, word: &str) -> bool {
        self.ignored_words.contains(&word.to_lowercase())
    }

    /// Appends a word to a `.docolint-ignore` file and adds it to the in-memory set.
    ///
    /// Creates the file if it does not exist. The word is lowercased before writing.
    /// No duplicate check is performed on the file; duplicates are harmless since
    /// the in-memory set deduplicates automatically.
    ///
    /// # Arguments
    /// * `word` - The word to ignore. Empty strings are silently ignored.
    /// * `target_file` - Path to the `.docolint-ignore` file to append to.
    ///
    /// # Errors
    /// Returns `std::io::Error` if the file cannot be opened or written.
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

    /// Filters out grammar errors whose matched word is in the ignored set.
    ///
    /// Extracts the word from `text` using each error's `offset` and `length`,
    /// then checks it against the ignored set. Errors with out-of-bounds offsets
    /// are kept (not filtered).
    ///
    /// # Arguments
    /// * `text` - The plain text string that LanguageTool checked. Offsets in errors
    ///   are relative to this string.
    /// * `errors` - Grammar errors to filter. Consumed by this function.
    ///
    /// # Returns
    /// A new `Vec` containing only errors whose matched word is not ignored.
    pub fn filter_errors(&self, text: &str, errors: Vec<GrammarError>) -> Vec<GrammarError> {
        errors.into_iter().filter(|error| {
            let Some(start) = Self::char_offset_to_byte_offset(text, error.offset) else {
                return true;
            };
            let Some(end) = Self::char_offset_to_byte_offset(text, error.offset + error.length) else {
                return true;
            };
            let Some(word) = text.get(start..end) else {
                return true;
            };
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
        
        let mut root_ignore = File::create(root_path.join(".docolint-ignore")).unwrap();
        writeln!(root_ignore, "rootword").unwrap();
        
        let mut sub_ignore = File::create(sub.join(".docolint-ignore")).unwrap();
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
        let ignore_file = root_path.join(".docolint-ignore");
        
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

    #[test]
    fn test_filter_errors_handles_unicode_offsets() {
        let mut dict = Dictionary::new();
        dict.ignored_words.insert("❌".to_string());

        let text = "alpha ❌ beta";
        let errors = vec![GrammarError {
            message: "Error".to_string(),
            offset: 6,
            length: 1,
            replacements: vec![],
            rule_id: "RULE1".to_string(),
        }];

        let filtered = dict.filter_errors(text, errors);
        assert!(filtered.is_empty());
    }
}
