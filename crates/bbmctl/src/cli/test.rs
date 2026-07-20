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

use clap::{ArgGroup, Args};

use super::ListArgs;

#[derive(Args, Clone)]
// `--provider`/`--plan` are only meaningful when the result is stored. Both
// `--record` and `--every` cause storing, so either must satisfy them --
// requiring `record` by name made `--every --provider` impossible to invoke.
#[command(group = ArgGroup::new("recording").args(["record", "every"]).multiple(true))]
pub struct TestArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Test duration per phase in seconds
    #[arg(long, default_value = "10")]
    pub duration: u64,

    /// Number of concurrent streams for throughput measurement
    #[arg(long, default_value = "8")]
    pub streams: u16,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,

    /// Measurement peer hostname (IPv4)
    #[arg(long)]
    pub peer: Option<String>,

    /// Run repeatedly at this interval (e.g. 30m, 1h, 2h30m). Implies --record.
    #[arg(long)]
    pub every: Option<String>,

    /// Also record the result to the local database
    #[arg(long)]
    pub record: bool,

    /// Provider ID to associate with the recorded measurement
    #[arg(long, requires = "recording")]
    pub provider: Option<i64>,

    /// Plan ID to associate with the recorded measurement
    #[arg(long, requires = "recording")]
    pub plan: Option<String>,
}

impl TestArgs {
    /// Whether this invocation stores its result. `--every` implies it, since
    /// a scheduled run that discarded every result would be pointless.
    pub fn records_to_database(&self) -> bool {
        self.record || self.every.is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    /// `--every` documents "Implies --record", but `--provider` carries
    /// `requires = "record"`, so the scheduled mode cannot be invoked with a
    /// provider without also passing the redundant `--record`.
    #[test]
    fn every_with_provider_is_accepted_without_explicit_record() {
        let cli = parse(&[
            "bbmctl",
            "test",
            "--every",
            "30m",
            "--provider",
            "437",
            "--duration",
            "1",
        ])
        .map_err(|e| e.to_string())
        .expect("--every implies --record, so --provider must be accepted");

        let Commands::Test(args) = cli.command else {
            panic!("expected the test subcommand");
        };
        assert_eq!(args.provider, Some(437));
        assert_eq!(args.every.as_deref(), Some("30m"));
    }

    /// `--every` on its own must still work.
    #[test]
    fn every_alone_is_accepted() {
        let cli = parse(&["bbmctl", "test", "--every", "30m"])
            .map_err(|e| e.to_string())
            .expect("--every alone must parse");
        let Commands::Test(args) = cli.command else {
            panic!("expected the test subcommand");
        };
        assert!(args.records_to_database(), "--every must imply recording");
    }

    /// Without `--every`, `--provider` still requires an explicit `--record`:
    /// a provider is only meaningful when something is being stored.
    #[test]
    fn provider_without_record_or_every_is_rejected() {
        let kind = parse(&["bbmctl", "test", "--provider", "437"])
            .err()
            .map(|e| e.kind());
        assert_eq!(
            kind,
            Some(clap::error::ErrorKind::MissingRequiredArgument),
            "--provider alone must be rejected"
        );
    }

    /// A plain run records nothing.
    #[test]
    fn plain_test_does_not_record() {
        let cli = parse(&["bbmctl", "test"]).unwrap();
        let Commands::Test(args) = cli.command else {
            panic!("expected the test subcommand");
        };
        assert!(!args.records_to_database());
    }
}
