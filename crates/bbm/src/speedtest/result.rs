use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SpeedTestResult {
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub latency_ms: f64,
    pub jitter_ms: f64,
    pub peer: String,
    pub duration_secs: u64,
    pub streams: u16,
}
