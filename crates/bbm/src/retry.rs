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
use std::time::Duration;

use tower::retry::Policy;

use crate::error::BbmError;

/// Default maximum number of retry attempts.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

/// A tower retry policy for transient HTTP failures with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub current_attempt: u32,
}

impl RetryPolicy {
    /// Create a new retry policy with the given maximum number of attempts.
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            current_attempt: 0,
        }
    }

    /// Calculate the backoff duration for the current attempt using exponential
    /// backoff: 200ms * 2^(current_attempt - 1).
    pub fn backoff_duration(&self) -> Duration {
        let base = Duration::from_millis(200);
        let factor = 2u64.saturating_pow(self.current_attempt.saturating_sub(1));
        base * factor as u32
    }

    /// Determine whether the given error is retryable.
    ///
    /// Retries on `BbmError::Http` when the underlying reqwest error is a
    /// timeout, connect error, or a 5xx server error, and on `BbmError::Io`.
    /// Does NOT retry `Api`, `Json`, or `TestFailed`.
    pub fn is_retryable(err: &BbmError) -> bool {
        match err {
            BbmError::Http(e) => {
                if e.is_timeout() || e.is_connect() {
                    return true;
                }
                if let Some(status) = e.status() {
                    return status.is_server_error();
                }
                false
            }
            BbmError::Io(_) => true,
            BbmError::Api(_) | BbmError::Json(_) | BbmError::TestFailed(_) => false,
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ATTEMPTS)
    }
}

impl<Req, Res> Policy<Req, Res, BbmError> for RetryPolicy
where
    Req: Clone,
{
    type Future = Pin<Box<dyn Future<Output = ()> + Send>>;

    fn retry(
        &mut self,
        _req: &mut Req,
        result: &mut Result<Res, BbmError>,
    ) -> Option<Self::Future> {
        match result {
            Ok(_) => None,
            Err(err) => {
                if !Self::is_retryable(err) {
                    return None;
                }

                if self.current_attempt >= self.max_attempts {
                    return None;
                }

                self.current_attempt += 1;
                let backoff = self.backoff_duration();

                log::warn!(
                    "Retryable error (attempt {}/{}), retrying after {:?}: {}",
                    self.current_attempt,
                    self.max_attempts,
                    backoff,
                    err,
                );

                Some(Box::pin(async move {
                    tokio::time::sleep(backoff).await;
                }))
            }
        }
    }

    fn clone_request(&mut self, req: &Req) -> Option<Req> {
        Some(req.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn retryable_errors() {
        // Io errors are retryable
        let io_err = BbmError::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "connection reset",
        ));
        assert!(RetryPolicy::is_retryable(&io_err));

        // Api errors are not retryable
        let api_err = BbmError::Api("bad request".into());
        assert!(!RetryPolicy::is_retryable(&api_err));

        // TestFailed errors are not retryable
        let test_err = BbmError::TestFailed("test failed".into());
        assert!(!RetryPolicy::is_retryable(&test_err));

        // Json errors are not retryable
        let json_err: BbmError = serde_json::from_str::<serde_json::Value>("not json")
            .unwrap_err()
            .into();
        assert!(!RetryPolicy::is_retryable(&json_err));

        // Http errors: we cannot easily construct specific reqwest::Error
        // variants (timeout, connect, 5xx) in unit tests since reqwest does
        // not expose constructors. We test the Http branch by making a request
        // with an invalid URL to get a builder error, which should not be retryable.
        let client = reqwest::Client::new();
        let req_err = client.get("not a url").build().unwrap_err();
        let http_err = BbmError::Http(req_err);
        // A builder/URL error is not retryable (no timeout/connect/5xx)
        assert!(!RetryPolicy::is_retryable(&http_err));
    }

    #[test]
    fn backoff_durations() {
        // attempt 0: 200ms * 2^(0-1) but saturating_sub gives 0, so 2^0 = 1 => 200ms
        let p0 = RetryPolicy {
            max_attempts: 3,
            current_attempt: 0,
        };
        assert_eq!(p0.backoff_duration(), Duration::from_millis(200));

        // attempt 1: 200ms * 2^0 = 200ms
        let p1 = RetryPolicy {
            max_attempts: 3,
            current_attempt: 1,
        };
        assert_eq!(p1.backoff_duration(), Duration::from_millis(200));

        // attempt 2: 200ms * 2^1 = 400ms
        let p2 = RetryPolicy {
            max_attempts: 3,
            current_attempt: 2,
        };
        assert_eq!(p2.backoff_duration(), Duration::from_millis(400));

        // attempt 3: 200ms * 2^2 = 800ms
        let p3 = RetryPolicy {
            max_attempts: 3,
            current_attempt: 3,
        };
        assert_eq!(p3.backoff_duration(), Duration::from_millis(800));
    }
}
