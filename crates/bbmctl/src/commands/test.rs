use anyhow::Result;
use log::info;

use super::make_writer;
use crate::cli::TestArgs;
use crate::config::ResolvedConfig;
use crate::utils::duration;
use crate::utils::output;
use crate::utils::speed_fmt::FormattedSpeedTestResult;
use bbmctl_database::Database;

pub async fn run(args: TestArgs, config: &ResolvedConfig, db: &Database) -> Result<()> {
    if let Some(every) = args.every.clone() {
        return run_scheduled(args, config, db, &every).await;
    }

    run_once(&args, config, db).await
}

async fn run_once(args: &TestArgs, config: &ResolvedConfig, db: &Database) -> Result<()> {
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

    if args.record || args.every.is_some() {
        let provider = args.provider.or(config.provider)
            .ok_or_else(|| anyhow::anyhow!("--provider is required (set it in config or pass explicitly)"))?;
        let m = db.measurements().record(
            result.download_kbps,
            result.upload_kbps,
            result.latency_ms,
            Some(provider),
            args.plan.as_deref().or(config.plan.as_deref()),
        ).await?;
        info!("recorded measurement #{} at {}", m.id, m.timestamp);
    }

    let formatted = FormattedSpeedTestResult::from_result(&result, &args.unit);
    let mut writer = make_writer(&args.list)?;
    output::write_output(&mut writer, &[formatted], &args.list.format)?;

    Ok(())
}

async fn run_scheduled(args: TestArgs, config: &ResolvedConfig, db: &Database, every: &str) -> Result<()> {
    let interval = duration::parse_duration(every)?;
    let interval_display = duration::format_duration(interval);

    // --every implies recording, so provider is required
    let provider = args.provider.or(config.provider)
        .ok_or_else(|| anyhow::anyhow!("--provider is required for scheduled tests (set it in config or pass explicitly)"))?;

    info!("running speed tests every {interval_display} (Ctrl+C to stop)");

    let mut iteration = 0u64;

    loop {
        iteration += 1;

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

        let m = db.measurements().record(
            result.download_kbps,
            result.upload_kbps,
            result.latency_ms,
            Some(provider),
            args.plan.as_deref().or(config.plan.as_deref()),
        ).await?;
        info!("recorded measurement #{} at {}", m.id, m.timestamp);

        let formatted = FormattedSpeedTestResult::from_result(&result, &args.unit);
        let mut writer = make_writer(&args.list)?;
        output::write_output(&mut writer, &[formatted], &args.list.format)?;

        info!("test #{iteration} complete, next in {interval_display}");

        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = tokio::signal::ctrl_c() => {
                info!("stopping scheduled tests ({iteration} tests completed)");
                break;
            }
        }
    }

    Ok(())
}
