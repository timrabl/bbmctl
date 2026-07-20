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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::info;
use reqwest::Client;

use crate::error::{BbmError, Result};

use super::{SpeedTestConfig, SpeedTestResult};

const UPLOAD_CHUNK_SIZE: usize = 131_072;
const LATENCY_SAMPLES: u32 = 10;

/// Runs speed tests (download, upload, latency) against a configured peer.
pub struct SpeedTestRunner {
    http: Client,
    config: SpeedTestConfig,
    url: String,
}

impl SpeedTestRunner {
    /// Create a new runner with the given configuration.
    pub fn new(config: SpeedTestConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.duration_secs + 5))
            .http1_only()
            .build()
            .map_err(BbmError::Http)?;
        // Honour an explicit scheme so a peer can be addressed over plain HTTP
        // or on a non-default port; otherwise default to HTTPS.
        let url = if config.peer.contains("://") {
            format!("{}/", config.peer.trim_end_matches('/'))
        } else {
            format!("https://{}/", config.peer)
        };
        Ok(Self { http, config, url })
    }

    /// Run the full speed test suite (latency, download, upload).
    pub async fn run(&self) -> Result<SpeedTestResult> {
        info!("measuring latency against {} ...", self.config.rtt_peer);
        let (latency_ms, jitter_ms) = self.measure_latency().await?;
        info!("  latency: {latency_ms:.1} ms (jitter: {jitter_ms:.1} ms)");

        info!(
            "measuring download against {} ({} seconds, {} streams) ...",
            self.config.peer, self.config.duration_secs, self.config.streams
        );
        let download_kbps = self.measure_download().await?;
        info!("  download: {:.2} Mbit/s", download_kbps / 1000.0);

        info!(
            "measuring upload against {} ({} seconds, {} streams) ...",
            self.config.peer, self.config.duration_secs, self.config.streams
        );
        let upload_kbps = self.measure_upload().await?;
        info!("  upload: {:.2} Mbit/s", upload_kbps / 1000.0);

        Ok(SpeedTestResult {
            download_kbps,
            upload_kbps,
            latency_ms,
            jitter_ms,
            peer: self.config.peer.clone(),
            duration_secs: self.config.duration_secs,
            streams: self.config.streams,
        })
    }

    /// Measure download throughput in kbit/s using concurrent streams.
    pub async fn measure_download(&self) -> Result<f64> {
        let total_bytes = Arc::new(AtomicU64::new(0));
        let start = Instant::now();
        let deadline = Duration::from_secs(self.config.duration_secs);

        let mut futures = FuturesUnordered::new();

        for _ in 0..self.config.streams {
            let http = self.http.clone();
            let url = self.url.clone();
            let bytes = Arc::clone(&total_bytes);

            futures.push(async move {
                while start.elapsed() < deadline {
                    let remaining = deadline.saturating_sub(start.elapsed());
                    if remaining.is_zero() {
                        break;
                    }
                    match http.get(&url).timeout(remaining).send().await {
                        Ok(mut response) => {
                            if !response.status().is_success() {
                                break;
                            }
                            // Count each chunk as it arrives. Buffering the
                            // whole body via `bytes()` would discard every
                            // byte of a request the deadline cuts off, and
                            // would hold the entire payload in memory per
                            // stream.
                            // Ends on body completion (`Ok(None)`) or a mid-
                            // stream error; either way whatever arrived has
                            // already been counted.
                            while let Ok(Some(chunk)) = response.chunk().await {
                                bytes.fetch_add(chunk.len() as u64, Ordering::Relaxed);
                                if start.elapsed() >= deadline {
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        while futures.next().await.is_some() {}

        // Check "no data" before "too short": when every stream fails
        // immediately both are true, and the transfer failure is the useful
        // diagnosis.
        let bytes = total_bytes.load(Ordering::Relaxed);
        if bytes == 0 {
            return Err(BbmError::TestFailed(
                "download moved no data (peer unreachable or rejecting requests)".into(),
            ));
        }

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed < 0.1 {
            return Err(BbmError::TestFailed(
                "download measurement too short".into(),
            ));
        }

        Ok((bytes as f64 * 8.0) / 1000.0 / elapsed)
    }

    /// Measure upload throughput in kbit/s using concurrent streams.
    pub async fn measure_upload(&self) -> Result<f64> {
        let total_bytes = Arc::new(AtomicU64::new(0));
        let start = Instant::now();
        let deadline = Duration::from_secs(self.config.duration_secs);

        let mut futures = FuturesUnordered::new();

        for _ in 0..self.config.streams {
            let http = self.http.clone();
            let url = self.url.clone();
            let bytes = Arc::clone(&total_bytes);

            futures.push(async move {
                let chunk = vec![0u8; UPLOAD_CHUNK_SIZE];
                while start.elapsed() < deadline {
                    let remaining = deadline.saturating_sub(start.elapsed());
                    if remaining.is_zero() {
                        break;
                    }
                    match http
                        .post(&url)
                        .timeout(remaining)
                        .body(chunk.clone())
                        .send()
                        .await
                    {
                        // `send()` resolves to `Ok` for any status, including
                        // 4xx/5xx. Only a request the peer actually accepted
                        // represents transferred payload.
                        Ok(resp) if resp.status().is_success() => {
                            let _ = resp.bytes().await;
                            bytes.fetch_add(UPLOAD_CHUNK_SIZE as u64, Ordering::Relaxed);
                        }
                        Ok(_) => break,
                        Err(_) => break,
                    }
                }
            });
        }

        while futures.next().await.is_some() {}

        let bytes = total_bytes.load(Ordering::Relaxed);
        if bytes == 0 {
            return Err(BbmError::TestFailed(
                "upload moved no data (peer unreachable or rejecting requests)".into(),
            ));
        }

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed < 0.1 {
            return Err(BbmError::TestFailed("upload measurement too short".into()));
        }

        Ok((bytes as f64 * 8.0) / 1000.0 / elapsed)
    }

    /// Measure latency and jitter in ms via async TCP connection timing.
    pub async fn measure_latency(&self) -> Result<(f64, f64)> {
        let addr = format!("{}:{}", self.config.rtt_peer, self.config.port);
        let resolved = tokio::net::lookup_host(&addr)
            .await?
            .next()
            .ok_or_else(|| BbmError::TestFailed(format!("could not resolve {addr}")))?;

        let mut rtts = Vec::with_capacity(LATENCY_SAMPLES as usize);

        for _ in 0..LATENCY_SAMPLES {
            let start = Instant::now();
            let stream = tokio::net::TcpStream::connect(resolved).await?;
            let elapsed = start.elapsed();
            drop(stream);
            rtts.push(elapsed.as_secs_f64() * 1000.0);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let avg = rtts.iter().sum::<f64>() / rtts.len() as f64;
        let jitter = if rtts.len() > 1 {
            let diffs: Vec<f64> = rtts.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
            diffs.iter().sum::<f64>() / diffs.len() as f64
        } else {
            0.0
        };

        Ok((avg, jitter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{http_response, StubServer};

    fn config_for(base_url: &str) -> SpeedTestConfig {
        SpeedTestConfig {
            peer: base_url.to_string(),
            rtt_peer: "127.0.0.1".to_string(),
            duration_secs: 1,
            port: 0,
            streams: 1,
        }
    }

    /// `reqwest::send()` returns `Ok` for any HTTP status. Crediting a full
    /// chunk for a 405 makes a server that rejects everything look fast --
    /// the faster it rejects, the higher the reported rate.
    #[tokio::test]
    async fn upload_does_not_count_rejected_requests() {
        let server = StubServer::serve_raw(http_response(405, "text/plain", "nope")).await;
        let runner = SpeedTestRunner::new(config_for(&server.base_url)).unwrap();

        let measured = runner.measure_upload().await;

        let rate = measured.unwrap_or(0.0);
        assert_eq!(
            rate, 0.0,
            "rejected uploads must not count toward throughput, got {rate} kbit/s"
        );
        assert!(server.hits() > 0, "stub server was never contacted");
    }

    /// Reporting "upload: 0.00 Mbit/s" as a successful result hides a total
    /// failure -- the caller cannot distinguish a broken peer from a dead line.
    #[tokio::test]
    async fn upload_that_transfers_nothing_is_an_error() {
        let server = StubServer::serve_raw(http_response(405, "text/plain", "nope")).await;
        let runner = SpeedTestRunner::new(config_for(&server.base_url)).unwrap();

        let err = runner
            .measure_upload()
            .await
            .expect_err("a measurement that moved no bytes must not be Ok");

        assert!(
            err.to_string().contains("no data"),
            "unexpected error: {err}"
        );
    }

    /// `response.bytes()` buffers the whole body before anything is counted,
    /// and the request timeout covers that read. A request cut off by the
    /// deadline therefore contributes zero bytes -- so when the payload is
    /// larger than one measurement window (a fast line hitting a real
    /// speedtest endpoint), no request ever completes and the measurement
    /// silently reports nothing transferred.
    #[tokio::test]
    async fn download_counts_bytes_from_requests_cut_off_by_the_deadline() {
        // 64 KiB every 20ms, body declared as 1 GiB: never finishes within 1s.
        let server = crate::testutil::serve_endless_stream(65_536, 20).await;
        let runner = SpeedTestRunner::new(config_for(&server.base_url)).unwrap();

        let kbps = runner
            .measure_download()
            .await
            .expect("partial transfers must still count as data");

        assert!(
            kbps > 0.0,
            "expected a positive rate from a partially transferred stream, got {kbps}"
        );
    }

    /// Same contract on the download side.
    #[tokio::test]
    async fn download_that_transfers_nothing_is_an_error() {
        let server = StubServer::serve_raw(http_response(500, "text/plain", "")).await;
        let runner = SpeedTestRunner::new(config_for(&server.base_url)).unwrap();

        let err = runner
            .measure_download()
            .await
            .expect_err("a measurement that moved no bytes must not be Ok");

        assert!(
            err.to_string().contains("no data"),
            "unexpected error: {err}"
        );
    }
}
