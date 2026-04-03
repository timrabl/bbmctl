use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub key: String,
    pub operator: String,
    pub company: String,
    pub value: String,
    #[serde(rename = "istop")]
    pub is_top: i64,
    #[serde(rename = "hasPlans")]
    pub has_plans: i64,
}

#[cfg(test)]
mod tests {
    use crate::client::BbmClient;

    const TESTING_PROVIDER_IDS: &[i64] = &[1, 7, 10, 11, 244, 251, 330, 416, 709];

    #[tokio::test]
    #[ignore] // hits live API
    async fn test_get_providers() {
        let client = BbmClient::new();
        let providers = client.get_providers().await.unwrap();
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_provider_by_id() {
        let client = BbmClient::new();
        for &id in TESTING_PROVIDER_IDS {
            let provider = client.get_provider_by_id(id).await.unwrap();
            assert_eq!(provider.key, id.to_string());
        }
    }
}
