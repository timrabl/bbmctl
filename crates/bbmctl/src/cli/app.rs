// Copyright (c) 2023-2026 Tim Oliver Rabl
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use clap::{Parser, Subcommand};
use clap_complete::Shell;

use super::{
    CampaignCommands, CompareArgs, ExportCommands, HistoryCommands, ListCommands, ProviderCommands,
    ReportCommands, TestArgs,
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
