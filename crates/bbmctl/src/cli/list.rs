use clap::{Args, Subcommand};

use super::ListArgs;

#[derive(Subcommand)]
pub enum ListCommands {
    /// List the available providers
    Providers(ProvidersArgs),
    /// List the available speeds
    Speeds(SpeedsArgs),
    /// List plans for a provider
    Plans(PlansArgs),
}

#[derive(Args, Clone)]
pub struct ProvidersArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Filter providers by name (case-insensitive substring match)
    #[arg(long)]
    pub search: Option<String>,

    /// Show only top providers
    #[arg(long)]
    pub top: bool,
}

#[derive(Args, Clone)]
pub struct SpeedsArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Filter speeds by provider ID
    #[arg(long)]
    pub provider: Option<i64>,
}

#[derive(Args, Clone)]
pub struct PlansArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Provider ID (required)
    #[arg(long)]
    pub provider: i64,
}
