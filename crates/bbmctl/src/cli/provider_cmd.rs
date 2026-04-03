use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// Set the active provider
    Switch(ProviderSwitchArgs),
    /// Show the current active provider
    Show,
}

#[derive(Args, Clone)]
pub struct ProviderSwitchArgs {
    /// Provider ID to set as active
    pub id: i64,
}
