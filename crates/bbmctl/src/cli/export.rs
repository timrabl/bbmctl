use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum ExportCommands {
    /// Start a Prometheus metrics exporter
    Prometheus(PrometheusArgs),
}

#[derive(Args, Clone)]
pub struct PrometheusArgs {
    /// Port to listen on
    #[arg(short, long, default_value = "9090")]
    pub port: u16,
}
