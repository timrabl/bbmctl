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

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use reqwest::Client;
use tower::retry::Retry;
use tower::Service;

use crate::error::{BbmError, Result};
use crate::plan::Plan;
use crate::provider::Provider;
use crate::report::AnnualReportSummary;
use crate::retry::RetryPolicy;
use crate::speed::Speed;

const DEFAULT_BASE_URL: &str = "https://breitbandmessung.de";

/// A request to the API: just a full URL.
#[derive(Debug, Clone)]
struct ApiRequest {
    url: String,
}

/// Internal tower service that performs an async GET and returns the response bytes.
#[derive(Clone)]
struct ApiService {
    http: Client,
}

impl Service<ApiRequest> for ApiService {
    type Response = reqwest::Response;
    type Error = BbmError;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ApiRequest) -> Self::Future {
        let client = self.http.clone();
        Box::pin(async move {
            let response = client
                .get(&req.url)
                .header("Accept", "application/json")
                .send()
                .await?
                // reqwest only reports a status error via `Kind::Status`, which
                // `error_for_status` produces. Without this the retry policy
                // sees `Ok(response)` for a 503 and its 5xx branch is dead code.
                .error_for_status()?;
            Ok(response)
        })
    }
}

/// Client for the Breitbandmessung API.
pub struct BbmClient {
    http: Client,
    base_url: String,
    retry_policy: RetryPolicy,
}

impl BbmClient {
    /// Create a new client with the default base URL.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Create a new client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Set a custom retry policy.
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let svc = ApiService {
            http: self.http.clone(),
        };
        let mut retry_svc = Retry::new(self.retry_policy.clone(), svc);

        let request = ApiRequest { url };
        let response = retry_svc.call(request).await?;

        let status = response.status();
        if !status.is_success() {
            return Err(BbmError::Api(format!("{path} returned HTTP {status}")));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();

        if !content_type.contains("application/json") {
            let body = response.text().await.unwrap_or_default();
            // Take characters, not bytes: a byte-index cut can land inside a
            // multi-byte character and panic.
            let preview: String = body.chars().take(200).collect();
            return Err(BbmError::Api(format!(
                "{path} returned non-JSON response (Content-Type: {content_type}): {preview}"
            )));
        }

        let bytes = response.bytes().await?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }

    // -- Provider API --

    /// Fetch all providers.
    pub async fn get_providers(&self) -> Result<Vec<Provider>> {
        self.get_json("/api/provider").await
    }

    /// Fetch a single provider by ID.
    pub async fn get_provider_by_id(&self, id: i64) -> Result<Provider> {
        self.get_json(&format!("/api/provider/{id}")).await
    }

    // -- Plan API --

    /// Fetch plans for a specific provider.
    pub async fn get_plans_by_provider_id(&self, provider_id: i64) -> Result<Vec<Plan>> {
        self.get_json(&format!("/api/plans_desktop/{provider_id}"))
            .await
    }

    // -- Speed API --

    /// Fetch all speeds.
    pub async fn get_speeds(&self) -> Result<Vec<Speed>> {
        self.get_json("/api/speed").await
    }

    /// Fetch speeds for a specific provider.
    pub async fn get_speeds_by_provider_id(&self, id: i64) -> Result<Vec<Speed>> {
        self.get_json(&format!("/api/speed/{id}")).await
    }

    // -- Statistics API --

    /// Fetch live statistics as raw JSON.
    pub async fn get_statistics(&self) -> Result<serde_json::Value> {
        self.get_json("/api/statistics").await
    }

    // -- Report data --

    /// Reference data from published BNetzA annual reports (static, not from the API).
    pub fn annual_reports() -> Vec<AnnualReportSummary> {
        crate::report::annual_reports()
    }
}

impl Default for BbmClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{http_response, StubServer};

    /// A German error page is full of multi-byte characters, so the 200th byte
    /// of a non-JSON body regularly lands mid-character. Slicing on a raw byte
    /// index panics there.
    #[tokio::test]
    async fn non_json_body_with_multibyte_char_at_cutoff_does_not_panic() {
        // 199 ASCII bytes, then 'ä' occupying bytes 199..201. A cut at byte 200
        // is not a character boundary.
        let body = format!("{}ä", "a".repeat(199));
        assert!(!body.is_char_boundary(200), "test fixture is wrong");

        let server = StubServer::serve_raw(http_response(200, "text/html", &body)).await;
        let client = BbmClient::with_base_url(&server.base_url);

        let err = client
            .get_providers()
            .await
            .expect_err("non-JSON response must be an error");

        assert!(
            err.to_string().contains("non-JSON"),
            "unexpected error: {err}"
        );
    }

    /// `RetryPolicy` documents retrying 5xx, but reqwest only reports a status
    /// error when `error_for_status` is called. Without it the retry layer sees
    /// `Ok(response)` and never fires.
    #[tokio::test]
    async fn server_errors_are_retried() {
        let server =
            StubServer::serve_raw(http_response(503, "application/json", r#"{"err":1}"#)).await;
        let client = BbmClient::with_base_url(&server.base_url);

        let err = client
            .get_providers()
            .await
            .expect_err("503 must surface as an error");

        // Default policy is 3 attempts: the initial call plus 3 retries.
        assert_eq!(
            server.hits(),
            4,
            "expected 4 attempts (1 initial + 3 retries), got {}: {err}",
            server.hits()
        );
    }

    /// A 404 is a client error and must not be retried.
    #[tokio::test]
    async fn client_errors_are_not_retried() {
        let server =
            StubServer::serve_raw(http_response(404, "application/json", r#"{"err":1}"#)).await;
        let client = BbmClient::with_base_url(&server.base_url);

        let _ = client.get_providers().await.expect_err("404 must error");

        assert_eq!(server.hits(), 1, "404 must not be retried");
    }
}
