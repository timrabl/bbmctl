use clap::Args;

use super::OutputFormat;

#[derive(Args, Clone)]
pub struct ListArgs {
    /// Output format
    #[arg(short = 'f', long, default_value = "table")]
    pub format: OutputFormat,

    /// Output file (default: stdout)
    #[arg(short, long)]
    pub output: Option<String>,
}
