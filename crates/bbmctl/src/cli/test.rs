use clap::Args;

use super::ListArgs;

#[derive(Args, Clone)]
pub struct TestArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Test duration per phase in seconds
    #[arg(long, default_value = "10")]
    pub duration: u64,

    /// Number of concurrent streams for throughput measurement
    #[arg(long, default_value = "8")]
    pub streams: u16,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,

    /// Measurement peer hostname (IPv4)
    #[arg(long)]
    pub peer: Option<String>,

    /// Run repeatedly at this interval (e.g. 30m, 1h, 2h30m). Implies --record.
    #[arg(long)]
    pub every: Option<String>,

    /// Also record the result to the local database
    #[arg(long)]
    pub record: bool,

    /// Provider ID to associate with the recorded measurement
    #[arg(long, requires = "record")]
    pub provider: Option<i64>,

    /// Plan ID to associate with the recorded measurement
    #[arg(long, requires = "record")]
    pub plan: Option<String>,
}
