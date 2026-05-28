use serde::Deserialize;

pub use docolint_types::{AnnotatedText, GrammarError, TextSegment};

/// Configuration for creating a [`LanguageToolClient`].
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the LanguageTool server (e.g., `http://localhost:8081`).
    /// Do not include the `/v2/check` path; it is appended automatically.
    pub base_url: String,
    /// LanguageTool language code (e.g., `en-US`).
    pub language: String,
    /// Disables the derived LanguageTool dictionary spelling rule for `language`.
    pub disable_spell_check: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8081".to_string(),
            language: "en-US".to_string(),
            disable_spell_check: false,
        }
    }
}

/// HTTP client for communicating with the LanguageTool API.
///
/// Wraps a `reqwest::Client` and handles serialization of [`AnnotatedText`]
/// into the format expected by LanguageTool's `/v2/check` endpoint. Deserializes
/// responses into [`GrammarError`] instances.
#[derive(Debug, Clone)]
pub struct LanguageToolClient {
    config: ClientConfig,
    client: reqwest::Client,
}

impl LanguageToolClient {
    /// Creates a new client with the given configuration.
    ///
    /// # Arguments
    /// * `config` - Connection settings including the LanguageTool server URL.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Returns the base URL configured for this client.
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Sends text to the LanguageTool `/v2/check` endpoint and returns grammar errors.
    ///
    /// Automatically chooses between form-encoded plain text or JSON annotation format
    /// based on whether the input contains markup segments.
    ///
    /// # Arguments
    /// * `text` - The annotated text to check. Prose segments are checked; markup
    ///   segments are skipped but preserved for offset mapping.
    ///
    /// # Errors
    /// Returns `reqwest::Error` on network failure, invalid URL, or deserialization failure.
    pub async fn check(&self, text: AnnotatedText) -> Result<Vec<GrammarError>, reqwest::Error> {
        let url = format!("{}/v2/check", self.config.base_url);
        let has_markup = text.segments.iter().any(|s| s.is_markup);
        let spelling_rule = self
            .config
            .disable_spell_check
            .then(|| spelling_rule_id(&self.config.language));

        let mut params: Vec<(&str, String)> = if has_markup {
            let data = serde_json::json!({ "annotation": text.segments });
            vec![
                ("language", self.config.language.clone()),
                ("data", data.to_string()),
            ]
        } else {
            vec![
                ("language", self.config.language.clone()),
                ("text", text.plain_text()),
            ]
        };

        if let Some(rule) = spelling_rule {
            params.push(("disabledRules", rule));
        }

        let resp = self.client.post(&url).form(&params).send().await?;
        let lt_resp: LTCheckResponse = resp.json().await?;
        Ok(lt_resp
            .matches
            .into_iter()
            .map(GrammarError::from)
            .collect())
    }
}

fn spelling_rule_id(language: &str) -> String {
    format!(
        "MORFOLOGIK_RULE_{}",
        language.replace('-', "_").to_uppercase()
    )
}

#[derive(Deserialize)]
struct LTCheckResponse {
    matches: Vec<LTMatch>,
}

#[derive(Deserialize)]
struct LTMatch {
    message: String,
    offset: usize,
    length: usize,
    replacements: Vec<LTReplacement>,
    rule: LTRule,
}

#[derive(Deserialize)]
struct LTReplacement {
    value: String,
}

#[derive(Deserialize)]
struct LTRule {
    id: String,
}

impl From<LTMatch> for GrammarError {
    fn from(m: LTMatch) -> Self {
        GrammarError {
            message: m.message,
            offset: m.offset,
            length: m.length,
            replacements: m.replacements.into_iter().map(|r| r.value).collect(),
            rule_id: m.rule.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_new_with_localhost() {
        let config = ClientConfig {
            base_url: "http://localhost:8081".to_string(),
            ..Default::default()
        };
        let client = LanguageToolClient::new(config);
        assert_eq!(client.base_url(), "http://localhost:8081");
    }

    #[test]
    fn test_new_with_cloud_url() {
        let config = ClientConfig {
            base_url: "https://api.languagetoolplus.com".to_string(),
            ..Default::default()
        };
        let client = LanguageToolClient::new(config);
        assert_eq!(client.base_url(), "https://api.languagetoolplus.com");
    }

    #[test]
    fn test_spelling_rule_id_uses_language() {
        assert_eq!(spelling_rule_id("en-US"), "MORFOLOGIK_RULE_EN_US");
        assert_eq!(spelling_rule_id("en-AU"), "MORFOLOGIK_RULE_EN_AU");
    }

    #[tokio::test]
    async fn test_check_multiple_matches() {
        let mock_server = MockServer::start().await;

        let sample_response = serde_json::json!({
            "matches": [
                {
                    "message": "Possible spelling mistake found.",
                    "shortMessage": "Spelling mistake",
                    "replacements": [{"value": "teh"}, {"value": "the"}],
                    "offset": 5,
                    "length": 3,
                    "rule": {"id": "MORFOLOGIK_RULE_EN_US"}
                },
                {
                    "message": "A grammatical problem.",
                    "shortMessage": "Grammar",
                    "replacements": [{"value": "was"}],
                    "offset": 20,
                    "length": 4,
                    "rule": {"id": "SOME_GRAMMAR_RULE"}
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/v2/check"))
            .and(body_string_contains("language=en-US"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sample_response))
            .mount(&mock_server)
            .await;

        let config = ClientConfig {
            base_url: mock_server.uri(),
            ..Default::default()
        };
        let client = LanguageToolClient::new(config);
        let text = AnnotatedText::from("some tezt with a agrammatical isue.");
        let errors = client.check(text).await.unwrap();

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].rule_id, "MORFOLOGIK_RULE_EN_US");
        assert_eq!(errors[0].offset, 5);
        assert_eq!(errors[0].replacements, vec!["teh", "the"]);
        assert_eq!(errors[1].rule_id, "SOME_GRAMMAR_RULE");
        assert_eq!(errors[1].offset, 20);
        assert_eq!(errors[1].replacements, vec!["was"]);
    }

    #[tokio::test]
    async fn test_check_annotated_text() {
        let mock_server = MockServer::start().await;

        let sample_response = serde_json::json!({
            "matches": [{
                "message": "Possible spelling mistake found.",
                "shortMessage": "Spelling mistake",
                "replacements": [{"value": "world"}],
                "offset": 6,
                "length": 5,
                "rule": {"id": "MORFOLOGIK_RULE_EN_US"}
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v2/check"))
            .and(body_string_contains("language=en-AU"))
            .and(body_string_contains("disabledRules=MORFOLOGIK_RULE_EN_AU"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sample_response))
            .mount(&mock_server)
            .await;

        let config = ClientConfig {
            base_url: mock_server.uri(),
            language: "en-AU".to_string(),
            disable_spell_check: true,
        };
        let client = LanguageToolClient::new(config);

        let text = AnnotatedText {
            segments: vec![
                TextSegment {
                    text: "Hello ".to_string(),
                    is_markup: false,
                    offset: 0,
                },
                TextSegment {
                    text: "<b>".to_string(),
                    is_markup: true,
                    offset: 6,
                },
                TextSegment {
                    text: "wurld".to_string(),
                    is_markup: false,
                    offset: 9,
                },
                TextSegment {
                    text: "</b>".to_string(),
                    is_markup: true,
                    offset: 14,
                },
                TextSegment {
                    text: "!".to_string(),
                    is_markup: false,
                    offset: 18,
                },
            ],
        };

        let errors = client.check(text).await.unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].offset, 6);
        assert_eq!(errors[0].length, 5);
        assert_eq!(errors[0].replacements, vec!["world"]);
    }

    #[tokio::test]
    async fn test_check_uses_configured_language() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/check"))
            .and(body_string_contains("language=en-AU"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "matches": []
            })))
            .mount(&mock_server)
            .await;

        let client = LanguageToolClient::new(ClientConfig {
            base_url: mock_server.uri(),
            language: "en-AU".to_string(),
            ..Default::default()
        });

        let errors = client.check(AnnotatedText::from("colour")).await.unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_check_disables_derived_spelling_rule() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/check"))
            .and(body_string_contains("language=en-AU"))
            .and(body_string_contains("disabledRules=MORFOLOGIK_RULE_EN_AU"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "matches": []
            })))
            .mount(&mock_server)
            .await;

        let client = LanguageToolClient::new(ClientConfig {
            base_url: mock_server.uri(),
            language: "en-AU".to_string(),
            disable_spell_check: true,
        });

        let errors = client.check(AnnotatedText::from("colour")).await.unwrap();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_plain_text_extraction() {
        let text = AnnotatedText {
            segments: vec![
                TextSegment {
                    text: "Hello ".to_string(),
                    is_markup: false,
                    offset: 0,
                },
                TextSegment {
                    text: "<b>".to_string(),
                    is_markup: true,
                    offset: 6,
                },
                TextSegment {
                    text: "world".to_string(),
                    is_markup: false,
                    offset: 9,
                },
                TextSegment {
                    text: "</b>".to_string(),
                    is_markup: true,
                    offset: 14,
                },
                TextSegment {
                    text: "!".to_string(),
                    is_markup: false,
                    offset: 18,
                },
            ],
        };
        assert_eq!(text.plain_text(), "Hello world!");
    }

    #[test]
    fn test_from_str() {
        let text = AnnotatedText::from("hello");
        assert_eq!(text.segments.len(), 1);
        assert!(!text.segments[0].is_markup);
        assert_eq!(text.segments[0].text, "hello");
    }
}
