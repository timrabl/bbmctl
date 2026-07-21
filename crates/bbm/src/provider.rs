// Copyright (c) 2023-2026 Tim Oliver Rabl
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
        crate::testutil::assert_graceful(client.get_providers().await, |providers| {
            assert!(!providers.is_empty());
        });
    }

    #[tokio::test]
    #[ignore] // hits live API
    async fn test_get_provider_by_id() {
        let client = BbmClient::new();
        for &id in TESTING_PROVIDER_IDS {
            crate::testutil::assert_graceful(client.get_provider_by_id(id).await, |provider| {
                assert_eq!(provider.key, id.to_string());
            });
        }
    }
}
