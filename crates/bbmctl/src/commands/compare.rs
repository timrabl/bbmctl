use anyhow::Result;
use log::info;

use super::make_writer;
use crate::cli::CompareArgs;
use crate::config::ResolvedConfig;
use crate::utils::output;
use crate::utils::speed_fmt::format_speed;
use bbmctl_database::Database;

/// Returns Ok(true) if all thresholds met, Ok(false) if any failed.
pub async fn run(args: CompareArgs, config: &ResolvedConfig, db: &Database) -> Result<bool> {
    let (download_kbps, upload_kbps) = if let (Some(dl), Some(ul)) = (args.download, args.upload) {
        (dl, ul)
    } else if args.test || (args.download.is_none() && args.upload.is_none()) {
        // Run a speed test
        let config_st = bbm::SpeedTestConfig {
            peer: config
                .peer
                .clone()
                .unwrap_or_else(|| bbm::SpeedTestConfig::DEFAULT_PEER.to_string()),
            ..Default::default()
        };
        let runner = bbm::SpeedTestRunner::new(config_st)?;
        let result = runner.run().await?;

        // Record if provider is available
        if let Some(provider) = config.provider {
            let m = db
                .measurements()
                .record(
                    result.download_kbps,
                    result.upload_kbps,
                    result.latency_ms,
                    Some(provider),
                    config.plan.as_deref(),
                )
                .await?;
            info!("recorded measurement #{} at {}", m.id, m.timestamp);
        }

        (result.download_kbps, result.upload_kbps)
    } else {
        anyhow::bail!("provide both --download and --upload, or use --test to run a speed test");
    };

    let client = bbm::BbmClient::new();
    let plans = client.get_plans_by_provider_id(args.provider).await?;

    if plans.is_empty() {
        anyhow::bail!("no plans found for provider {}", args.provider);
    }

    let target_plans: Vec<&bbm::Plan> = match &args.plan {
        Some(plan_id) => {
            let matched: Vec<_> = plans.iter().filter(|p| p.plan_id.0 == *plan_id).collect();
            if matched.is_empty() {
                anyhow::bail!(
                    "plan '{}' not found for provider {} — use `list plans --provider {}` to see available plans",
                    plan_id, args.provider, args.provider
                );
            }
            matched
        }
        None => plans.iter().collect(),
    };

    let results: Vec<bbm::ComparisonResult> = target_plans
        .iter()
        .map(|p| p.compare(download_kbps, upload_kbps))
        .collect();

    let rows = bbm::ComparisonResult::flatten(&results);
    let mut writer = make_writer(&args.list)?;
    output::write_output(&mut writer, &rows, &args.list.format)?;

    let all_passed = results.iter().all(|r| r.all_met);

    let unit = &args.unit;
    for r in &results {
        let status = if r.all_met { "ALL PASS" } else { "FAIL" };
        info!(
            "plan {} (dl: {}, ul: {}): {status}",
            r.plan_id,
            format_speed(download_kbps, unit),
            format_speed(upload_kbps, unit),
        );
    }

    Ok(all_passed)
}
