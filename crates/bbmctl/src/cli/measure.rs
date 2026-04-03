use clap::{Args, Subcommand};

use super::ListArgs;

#[derive(Subcommand)]
pub enum HistoryCommands {
    /// Manually add a measurement to the local database
    Add(AddArgs),
    /// Show measurement history
    List(HistoryListArgs),
    /// Show aggregate statistics
    Summary(SummaryArgs),
    /// Delete measurements
    Delete(DeleteArgs),
    /// Show speed trends as sparkline chart
    Trend(TrendArgs),
    /// Delete measurements older than a duration
    Purge(PurgeArgs),
    /// Export measurements to CSV
    Export(ExportArgs),
    /// Import measurements from CSV
    Import(ImportArgs),
}

#[derive(Args, Clone)]
pub struct AddArgs {
    /// Download speed in kbit/s
    #[arg(long)]
    pub download: f64,
    /// Upload speed in kbit/s
    #[arg(long)]
    pub upload: f64,
    /// Latency in ms
    #[arg(long)]
    pub latency: f64,
    /// Provider ID to associate with this measurement
    #[arg(long)]
    pub provider: Option<i64>,
    /// Plan ID to associate with this measurement
    #[arg(long)]
    pub plan: Option<String>,
}

#[derive(Args, Clone)]
pub struct HistoryListArgs {
    #[command(flatten)]
    pub list: ListArgs,
    /// Maximum number of measurements to show
    #[arg(short = 'n', long, default_value = "100")]
    pub limit: u32,
}

#[derive(Args, Clone)]
pub struct SummaryArgs {
    #[command(flatten)]
    pub list: ListArgs,
}

#[derive(Args, Clone)]
pub struct DeleteArgs {
    /// Measurement ID to delete
    pub id: Option<i64>,
    /// Delete all measurements
    #[arg(long)]
    pub all: bool,
    /// Confirm deletion (required with --all)
    #[arg(long, requires = "all")]
    pub confirm: bool,
}

#[derive(Args, Clone)]
pub struct PurgeArgs {
    /// Delete measurements older than this duration (e.g. 30d, 7d, 24h)
    #[arg(long)]
    pub older_than: String,
    /// Confirm the purge
    #[arg(long)]
    pub confirm: bool,
}

#[derive(Args, Clone)]
pub struct ExportArgs {
    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Args, Clone)]
pub struct ImportArgs {
    /// CSV file to import
    pub file: String,
}

#[derive(Args, Clone)]
pub struct TrendArgs {
    /// Number of measurements to include (default: 20)
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: u32,

    /// Time range to include (e.g. 24h, 7d, 1h30m)
    #[arg(long)]
    pub last: Option<String>,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,
}
