use clap::Args;

use super::ListArgs;

/// Compare measured speeds against a contractual plan
///
/// Exit codes: 0 = all thresholds met, 1 = error, 2 = one or more thresholds not met
#[derive(Args, Clone)]
pub struct CompareArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Provider ID
    #[arg(long)]
    pub provider: i64,

    /// Plan ID (if omitted, compares against all plans for the provider)
    #[arg(long)]
    pub plan: Option<String>,

    /// Measured download speed in kbit/s (omit to run a speed test)
    #[arg(long, requires = "upload")]
    pub download: Option<f64>,

    /// Measured upload speed in kbit/s (omit to run a speed test)
    #[arg(long, requires = "download")]
    pub upload: Option<f64>,

    /// Run a speed test before comparing (implied when --download/--upload omitted)
    #[arg(long)]
    pub test: bool,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,
}
