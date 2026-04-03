use anyhow::Result;
use log::warn;

use super::make_writer;
use crate::cli::ListCommands;
use crate::utils::output;

pub async fn run(command: ListCommands) -> Result<()> {
    let client = bbm::BbmClient::new();

    match command {
        ListCommands::Providers(args) => {
            let mut providers = client.get_providers().await?;

            if args.top {
                providers.retain(|p| p.is_top == 1);
                if providers.is_empty() {
                    warn!("--top filter returned no results (API currently returns istop=0 for all providers)");
                }
            }

            if let Some(ref query) = args.search {
                let query_lower = query.to_lowercase();
                providers.retain(|p| {
                    p.operator.to_lowercase().contains(&query_lower)
                        || p.company.to_lowercase().contains(&query_lower)
                        || p.value.to_lowercase().contains(&query_lower)
                });
            }

            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &providers, &args.list.format)?;
        }
        ListCommands::Speeds(args) => {
            warn!("speed endpoint is currently unstable — results may be unreliable");
            let speeds = match args.provider {
                Some(id) => client.get_speeds_by_provider_id(id).await?,
                None => client.get_speeds().await?,
            };
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &speeds, &args.list.format)?;
        }
        ListCommands::Plans(args) => {
            let plans = client.get_plans_by_provider_id(args.provider).await?;
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &plans, &args.list.format)?;
        }
    }

    Ok(())
}
