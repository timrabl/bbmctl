mod cli;
mod commands;
mod config;
mod prometheus;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use config::ConfigFile;
use bbmctl_database::Database;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,sqlx=warn,sea_orm=warn,sea_orm_migration=warn"),
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
    let resolved = config.resolve(cli.profile.as_deref())?;

    let db = Database::connect(resolved.database.as_deref()).await?;

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
