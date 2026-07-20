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

mod cli;
mod commands;
mod config;
mod prometheus;
mod utils;

use anyhow::Result;
use bbmctl_database::Database;
use clap::Parser;
use cli::{Cli, Commands};
use config::ConfigFile;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info,sqlx=warn,sea_orm=warn,sea_orm_migration=warn"),
    )
    .format_target(false)
    .format_timestamp(None)
    .init();

    let cli = Cli::parse();

    if let Commands::Completions { shell } = cli.command {
        commands::completions::run(shell);
        return Ok(());
    }

    let config = ConfigFile::load(cli.config.as_deref())?;
    let mut resolved = config.resolve(cli.profile.as_deref())?;

    let db = Database::connect(resolved.database.as_deref()).await?;

    // `provider switch` stores an active provider; fold it in now that the
    // database is open. It ranks below the CLI flag and config file, so it
    // only fills a gap -- but without this step it was written and never read
    // by anything except `provider show`.
    let stored_provider = db
        .settings()
        .get("active_provider")
        .await?
        .and_then(|v| v.parse::<i64>().ok());
    resolved.apply_stored_provider(stored_provider);

    match cli.command {
        Commands::Completions { .. } => unreachable!(),
        Commands::List { command } => commands::list::run(command).await?,
        Commands::History { command } => commands::history::run(command, &resolved, &db).await?,
        Commands::Campaign { command } => {
            let all_passed = commands::campaign::run(command, &resolved, &db).await?;
            if !all_passed {
                std::process::exit(2);
            }
        }
        Commands::Compare(args) => {
            let all_passed = commands::compare::run(args, &resolved, &db).await?;
            if !all_passed {
                std::process::exit(2);
            }
        }
        Commands::Export { command } => commands::export::run(command, &db).await?,
        Commands::Test(args) => commands::test::run(args, &resolved, &db).await?,
        Commands::Report { command } => commands::report::run(command).await?,
        Commands::Provider { command } => commands::provider::run(command, &resolved, &db).await?,
    }

    Ok(())
}

#[cfg(test)]
mod version_tests {
    /// All workspace crates must report one version.
    ///
    /// release-plz only bumps `bbm` (the others are `release = false`), so
    /// before the versions were inherited from `[workspace.package]` the
    /// v0.1.1 release shipped a binary reporting `bbmctl 0.1.0` and a
    /// `bbmctl_0.1.0-1_amd64.deb`. For apt that is worse than cosmetic: a
    /// genuine 0.1.0 package compares equal and dpkg refuses to upgrade.
    ///
    /// Both sides are compile-time constants, so reintroducing a literal
    /// `version = "..."` in either manifest fails the build.
    #[test]
    fn cli_and_library_versions_match() {
        assert_eq!(
            env!("CARGO_PKG_VERSION"),
            bbm::VERSION,
            "bbmctl and bbm must share the workspace version; \
             check that neither Cargo.toml has re-declared `version`"
        );
    }
}
