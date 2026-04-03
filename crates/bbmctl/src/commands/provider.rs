use anyhow::Result;
use bbmctl_database::Database;
use log::info;

use crate::config::ResolvedConfig;

pub async fn run(
    command: crate::cli::ProviderCommands,
    config: &ResolvedConfig,
    db: &Database,
) -> Result<()> {
    match command {
        crate::cli::ProviderCommands::Switch(args) => {
            db.settings()
                .set("active_provider", &args.id.to_string())
                .await?;
            info!("active provider set to {}", args.id);
        }
        crate::cli::ProviderCommands::Show => {
            let active = db.settings().get("active_provider").await?;
            match active {
                Some(id) => println!("active provider: {id} (from database)"),
                None => match config.provider {
                    Some(id) => println!("active provider: {id} (from config)"),
                    None => println!("no active provider set"),
                },
            }
        }
    }
    Ok(())
}
