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
        let url = format!("https://{}/", config.peer);
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
                        Ok(response) => match response.bytes().await {
                            Ok(b) => {
                                bytes.fetch_add(b.len() as u64, Ordering::Relaxed);
                            }
                            Err(_) => break,
                        },
                        Err(_) => break,
                    }
                }
            });
        }

        while futures.next().await.is_some() {}

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed < 0.1 {
            return Err(BbmError::TestFailed(
                "download measurement too short".into(),
            ));
        }

        let bytes = total_bytes.load(Ordering::Relaxed);
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
                        Ok(_) => {
                            bytes.fetch_add(UPLOAD_CHUNK_SIZE as u64, Ordering::Relaxed);
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        while futures.next().await.is_some() {}

        let elapsed = start.elapsed().as_secs_f64();
        if elapsed < 0.1 {
            return Err(BbmError::TestFailed(
                "upload measurement too short".into(),
            ));
        }

        let bytes = total_bytes.load(Ordering::Relaxed);
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
