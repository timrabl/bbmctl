use anyhow::Result;
use log::{info, warn};

use super::make_writer;
use crate::cli::CampaignCommands;
use crate::config::ResolvedConfig;
use crate::utils::output;
use crate::utils::speed_fmt::{format_speed, FormattedSpeedTestResult, SpeedUnit};
use bbmctl_database::Database;

/// Returns Ok(true) normally, Ok(false) if campaign test thresholds failed.
pub async fn run(command: CampaignCommands, config: &ResolvedConfig, db: &Database) -> Result<bool> {
    match command {
        CampaignCommands::Start(args) => {
            let c = db.campaigns().start(args.provider, &args.plan).await?;
            info!(
                "started campaign #{} for provider {} / plan {} at {}",
                c.id, c.provider_id, c.plan_id, c.started_at
            );
            info!("you have 14 days to complete 30 measurements across 3+ calendar days.");
        }
        CampaignCommands::Status(args) => {
            let campaign = db
                .campaigns()
                .active()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no active campaign"))?;
            let status = db.campaigns().status(campaign.id).await?;

            for w in &status.warnings {
                log::warn!("{w}");
            }

            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &[status], &args.list.format)?;
        }
        CampaignCommands::Add(args) => {
            let campaign = db
                .campaigns()
                .active()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no active campaign — start one with `campaign start`"))?;
            let m = db.campaigns().record(campaign.id, args.download, args.upload, args.latency).await?;
            info!("recorded campaign measurement #{} at {}", m.id, m.timestamp);
        }
        CampaignCommands::Report(args) => {
            let campaign = db
                .campaigns()
                .active()
                .await?
                .or(db.campaigns().latest().await?)
                .ok_or_else(|| anyhow::anyhow!("no campaign found"))?;
            let report = db.campaigns().report(campaign.id).await?;
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &[report], &args.list.format)?;
        }
        CampaignCommands::Cancel => {
            let campaign = db
                .campaigns()
                .active()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no active campaign to cancel"))?;
            db.campaigns().cancel(campaign.id).await?;
            info!("cancelled campaign #{}", campaign.id);
        }
        CampaignCommands::Test(args) => {
            let campaign = db
                .campaigns()
                .active()
                .await?
                .ok_or_else(|| anyhow::anyhow!("no active campaign — start one with `campaign start`"))?;

            let config_st = bbm::SpeedTestConfig {
                duration_secs: args.duration,
                streams: args.streams,
                peer: args
                    .peer
                    .clone()
                    .or_else(|| config.peer.clone())
                    .unwrap_or_else(|| bbm::SpeedTestConfig::DEFAULT_PEER.to_string()),
                ..Default::default()
            };

            let runner = bbm::SpeedTestRunner::new(config_st)?;
            let result = runner.run().await?;

            let m = db.campaigns().record(
                campaign.id,
                result.download_kbps,
                result.upload_kbps,
                result.latency_ms,
            ).await?;
            info!("recorded campaign measurement #{} at {}", m.id, m.timestamp);

            // Auto-compare against the campaign's plan
            let all_passed = match auto_compare(
                campaign.provider_id,
                &campaign.plan_id,
                result.download_kbps,
                result.upload_kbps,
                &args.unit,
            ).await {
                Ok(passed) => passed,
                Err(e) => {
                    warn!("could not compare against plan: {e}");
                    true // don't fail the recording
                }
            };

            let formatted = FormattedSpeedTestResult::from_result(&result, &args.unit);
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &[formatted], &args.list.format)?;

            return Ok(all_passed);
        }
        CampaignCommands::List(args) => {
            let campaigns = db.campaigns().list(args.status.as_deref()).await?;
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &campaigns, &args.list.format)?;
        }
    }

    Ok(true)
}

async fn auto_compare(
    provider_id: i64,
    plan_id: &str,
    download_kbps: f64,
    upload_kbps: f64,
    unit: &SpeedUnit,
) -> Result<bool> {
    let client = bbm::BbmClient::new();
    let plans = client.get_plans_by_provider_id(provider_id).await?;

    let plan = plans
        .iter()
        .find(|p| p.plan_id.0 == plan_id)
        .ok_or_else(|| anyhow::anyhow!("plan '{plan_id}' not found for provider {provider_id}"))?;

    let result = plan.compare(download_kbps, upload_kbps);

    // Log download thresholds
    let dl_checks: Vec<String> = result.results.iter()
        .filter(|r| r.check.starts_with("download"))
        .filter(|r| r.threshold_kbps.is_some())
        .map(|r| {
            let label = r.check.trim_start_matches("download >= ");
            let status = if r.met { "PASS" } else { "FAIL" };
            format!("{label}: {status} ({:.0}%)", r.percent)
        })
        .collect();

    info!(
        "download: {} — {}",
        format_speed(download_kbps, unit),
        dl_checks.join(", ")
    );

    // Log upload thresholds
    let ul_checks: Vec<String> = result.results.iter()
        .filter(|r| r.check.starts_with("upload"))
        .filter(|r| r.threshold_kbps.is_some())
        .map(|r| {
            let label = r.check.trim_start_matches("upload >= ");
            let status = if r.met { "PASS" } else { "FAIL" };
            format!("{label}: {status} ({:.0}%)", r.percent)
        })
        .collect();

    info!(
        "upload: {} — {}",
        format_speed(upload_kbps, unit),
        ul_checks.join(", ")
    );

    Ok(result.all_met)
}
