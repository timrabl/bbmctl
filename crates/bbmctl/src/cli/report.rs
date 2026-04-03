use clap::{Args, Subcommand};

use super::ListArgs;

#[derive(Subcommand)]
pub enum ReportCommands {
    /// Show Bundesnetzagentur annual report summaries
    Annual(ReportAnnualArgs),
    /// Fetch live statistics from the API
    Stats(ReportStatsArgs),
}

#[derive(Args, Clone)]
pub struct ReportAnnualArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Filter by year
    #[arg(long)]
    pub year: Option<u32>,
}

#[derive(Args, Clone)]
pub struct ReportStatsArgs {
    #[command(flatten)]
    pub list: ListArgs,
}
