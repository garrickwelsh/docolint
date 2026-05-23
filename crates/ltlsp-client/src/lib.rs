pub struct ClientConfig {
    pub base_url: String,
}

pub struct LanguageToolClient {
    config: ClientConfig,
}

impl LanguageToolClient {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }

    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
