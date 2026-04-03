use anyhow::Result;
use log::{info, warn};

use super::make_writer;
use crate::cli::ReportCommands;
use crate::utils::output;

pub async fn run(command: ReportCommands) -> Result<()> {
    match command {
        ReportCommands::Annual(args) => {
            let mut reports = bbm::BbmClient::annual_reports();
            if let Some(year) = args.year {
                reports.retain(|r| r.year == year);
            }
            if reports.is_empty() {
                info!("no report data found for the specified filter");
            } else {
                let mut writer = make_writer(&args.list)?;
                output::write_output(&mut writer, &reports, &args.list.format)?;
            }
        }
        ReportCommands::Stats(args) => {
            let client = bbm::BbmClient::new();
            match client.get_statistics().await {
                Ok(stats) => {
                    let mut writer = make_writer(&args.list)?;
                    output::write_output(&mut writer, &[stats], &args.list.format)?;
                }
                Err(e) => {
                    warn!("could not fetch live statistics: {e}");
                    info!("this endpoint may not be publicly available");
                    info!("use `report annual` for published report data instead");
                }
            }
        }
    }
    Ok(())
}
