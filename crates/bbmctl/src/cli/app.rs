use clap::{Parser, Subcommand};
use clap_complete::Shell;

use super::{
    CampaignCommands, CompareArgs, ExportCommands, HistoryCommands, ListCommands,
    ProviderCommands, ReportCommands, TestArgs,
};

#[derive(Parser)]
#[command(name = "bbmctl", about = "CLI for the Breitbandmessung API", version)]
pub struct Cli {
    /// Path to config file
    #[arg(long, global = true)]
    pub config: Option<String>,

    /// Named profile to use
    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List the available data from each endpoint
    List {
        #[command(subcommand)]
        command: ListCommands,
    },

    /// View and manage stored measurements
    History {
        #[command(subcommand)]
        command: HistoryCommands,
    },

    /// Manage Bundesnetzagentur measurement campaigns (Nachweisverfahren)
    Campaign {
        #[command(subcommand)]
        command: CampaignCommands,
    },

    /// Compare measured speeds against a contractual plan
    Compare(CompareArgs),

    /// Export metrics for monitoring systems
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },

    /// Run a broadband speed test
    Test(TestArgs),

    /// View Bundesnetzagentur broadband report data
    Report {
        #[command(subcommand)]
        command: ReportCommands,
    },

    /// Manage active provider
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}
