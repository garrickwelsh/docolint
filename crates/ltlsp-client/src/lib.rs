use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct GrammarError {
    pub message: String,
    pub offset: usize,
    pub length: usize,
    pub replacements: Vec<String>,
    pub rule_id: String,
}

pub struct ClientConfig {
    pub base_url: String,
}

pub struct LanguageToolClient {
    config: ClientConfig,
    client: reqwest::Client,
}

impl LanguageToolClient {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub async fn check(&self, text: &str) -> Result<Vec<GrammarError>, reqwest::Error> {
        let url = format!("{}/v2/check", self.config.base_url);
        let params = [("language", "en-US"), ("text", text)];
        let resp = self.client.post(&url).form(&params).send().await?;
        let lt_resp: LTCheckResponse = resp.json().await?;
        Ok(lt_resp.matches.into_iter().map(GrammarError::from).collect())
    }
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
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[test]
    fn test_new_with_localhost() {
        let config = ClientConfig {
            base_url: "http://localhost:8081".to_string(),
        };
        let client = LanguageToolClient::new(config);
        assert_eq!(client.base_url(), "http://localhost:8081");
    }

    #[test]
    fn test_new_with_cloud_url() {
        let config = ClientConfig {
            base_url: "https://api.languagetoolplus.com".to_string(),
        };
        let client = LanguageToolClient::new(config);
        assert_eq!(client.base_url(), "https://api.languagetoolplus.com");
    }

    #[tokio::test]
    async fn test_check_simple_string() {
        let mock_server = MockServer::start().await;

        let sample_response = serde_json::json!({
            "matches": [{
                "message": "Possible spelling mistake found.",
                "shortMessage": "Spelling mistake",
                "replacements": [{"value": "test"}],
                "offset": 10,
                "length": 5,
                "context": {"text": "This is a testt.", "offset": 10, "length": 5},
                "sentence": "This is a testt.",
                "type": {"typeName": "UnknownWord"},
                "rule": {
                    "id": "MORFOLOGIK_RULE_EN_US",
                    "description": "Possible spelling mistake",
                    "issueType": "misspelling",
                    "category": {"id": "TYPOS", "name": "Possible Typo"}
                },
                "ignoreForIncompleteSentence": false,
                "contextForSureMatch": 0
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v2/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&sample_response))
            .mount(&mock_server)
            .await;

        let config = ClientConfig {
            base_url: mock_server.uri(),
        };
        let client = LanguageToolClient::new(config);
        let errors = client.check("This is a testt.").await.unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "Possible spelling mistake found.");
        assert_eq!(errors[0].offset, 10);
        assert_eq!(errors[0].length, 5);
        assert_eq!(errors[0].replacements, vec!["test"]);
        assert_eq!(errors[0].rule_id, "MORFOLOGIK_RULE_EN_US");
    }
}
