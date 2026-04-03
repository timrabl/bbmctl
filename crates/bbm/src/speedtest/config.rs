#[derive(Debug, Clone)]
pub struct SpeedTestConfig {
    pub peer: String,
    pub rtt_peer: String,
    pub duration_secs: u64,
    pub port: u16,
    pub streams: u16,
}

impl SpeedTestConfig {
    pub const DEFAULT_PEER: &str = "drsfv4.breitbandmessung.de";
    pub const DEFAULT_RTT_PEER: &str = "drsfrtt.breitbandmessung.de";
    pub const DEFAULT_DURATION_SECS: u64 = 10;
    pub const DEFAULT_PORT: u16 = 443;
    pub const DEFAULT_STREAMS: u16 = 8;
}

impl Default for SpeedTestConfig {
    fn default() -> Self {
        Self {
            peer: Self::DEFAULT_PEER.to_string(),
            rtt_peer: Self::DEFAULT_RTT_PEER.to_string(),
            duration_secs: Self::DEFAULT_DURATION_SECS,
            port: Self::DEFAULT_PORT,
            streams: Self::DEFAULT_STREAMS,
        }
    }
}
