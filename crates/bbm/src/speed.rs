use serde::{Deserialize, Serialize};

use crate::intstr::InconsistentIntegerString;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Speed {
    pub key: String,
    pub value: String,
    #[serde(rename = "providerId")]
    pub provider_id: InconsistentIntegerString,
}

#[cfg(test)]
mod tests {
    use crate::client::BbmClient;

    const TESTING_PROVIDER_IDS: &[i64] = &[1, 7, 10, 11, 244, 251, 330, 416, 709];

    #[tokio::test]
    #[ignore]
    async fn test_get_speeds() {
        let client = BbmClient::new();
        let speeds = client.get_speeds().await.unwrap();
        assert!(!speeds.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_speeds_by_provider_id() {
        let client = BbmClient::new();
        for &id in TESTING_PROVIDER_IDS {
            let speeds = client.get_speeds_by_provider_id(id).await.unwrap();
            for speed in &speeds {
                assert!(!speed.key.is_empty());
            }
        }
    }
}
