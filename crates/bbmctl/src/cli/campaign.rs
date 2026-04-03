use clap::{Args, Subcommand};

use super::ListArgs;

#[derive(Subcommand)]
pub enum CampaignCommands {
    /// Start a new measurement campaign
    Start(CampaignStartArgs),
    /// Show current campaign status
    Status(CampaignStatusArgs),
    /// Add a measurement to the active campaign
    Add(CampaignAddArgs),
    /// Generate a campaign report
    Report(CampaignReportArgs),
    /// Cancel the active campaign
    Cancel,
    /// Run a speed test and record to the active campaign
    Test(CampaignTestArgs),
    /// List all campaigns
    List(CampaignListArgs),
}

#[derive(Args, Clone)]
pub struct CampaignStartArgs {
    /// Provider ID
    #[arg(long)]
    pub provider: i64,

    /// Plan ID
    #[arg(long)]
    pub plan: String,
}

#[derive(Args, Clone)]
pub struct CampaignStatusArgs {
    #[command(flatten)]
    pub list: ListArgs,
}

#[derive(Args, Clone)]
pub struct CampaignAddArgs {
    /// Download speed in kbit/s
    #[arg(long)]
    pub download: f64,

    /// Upload speed in kbit/s
    #[arg(long)]
    pub upload: f64,

    /// Latency in ms
    #[arg(long)]
    pub latency: f64,
}

#[derive(Args, Clone)]
pub struct CampaignReportArgs {
    #[command(flatten)]
    pub list: ListArgs,
}

#[derive(Args, Clone)]
pub struct CampaignTestArgs {
    #[command(flatten)]
    pub list: ListArgs,
    #[arg(long, default_value = "10")]
    pub duration: u64,
    #[arg(long, default_value = "8")]
    pub streams: u16,
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,
    #[arg(long)]
    pub peer: Option<String>,
}

#[derive(Args, Clone)]
pub struct CampaignListArgs {
    #[command(flatten)]
    pub list: ListArgs,
    #[arg(long)]
    pub status: Option<String>,
}
