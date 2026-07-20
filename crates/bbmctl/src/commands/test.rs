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

use anyhow::Result;
use log::{info, warn};

use super::make_writer;
use crate::cli::TestArgs;
use crate::config::ResolvedConfig;
use crate::utils::duration;
use crate::utils::output;
use crate::utils::speed_fmt::FormattedSpeedTestResult;
use bbmctl_database::Database;

/// Ticker for scheduled runs.
///
/// Uses `interval` rather than sleeping after each run, so the period is
/// measured start-to-start. Sleeping afterwards makes the real period
/// `interval + run_duration`, which slips by the length of every run.
/// `Delay` prevents a run that overruns its period from producing a burst of
/// immediate catch-up ticks.
fn scheduled_ticker(period: std::time::Duration) -> tokio::time::Interval {
    let mut ticker = tokio::time::interval(period);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker
}

pub async fn run(args: TestArgs, config: &ResolvedConfig, db: &Database) -> Result<()> {
    if let Some(every) = args.every.clone() {
        return run_scheduled(args, config, db, &every).await;
    }

    run_once(&args, config, db).await
}

async fn run_once(args: &TestArgs, config: &ResolvedConfig, db: &Database) -> Result<()> {
    let config_st = bbm::SpeedTestConfig {
        duration_secs: args.resolved_duration(config),
        streams: args.resolved_streams(config),
        peer: args
            .peer
            .clone()
            .or_else(|| config.peer.clone())
            .unwrap_or_else(|| bbm::SpeedTestConfig::DEFAULT_PEER.to_string()),
        ..Default::default()
    };

    let runner = bbm::SpeedTestRunner::new(config_st)?;
    let result = runner.run().await?;

    if args.records_to_database() {
        let provider = args.provider.or(config.provider).ok_or_else(|| {
            anyhow::anyhow!("--provider is required (set it in config or pass explicitly)")
        })?;
        let m = db
            .measurements()
            .record(
                result.download_kbps,
                result.upload_kbps,
                result.latency_ms,
                Some(provider),
                args.plan.as_deref().or(config.plan.as_deref()),
            )
            .await?;
        info!("recorded measurement #{} at {}", m.id, m.timestamp);
    }

    let formatted = FormattedSpeedTestResult::from_result(&result, &args.unit);
    let mut writer = make_writer(&args.list)?;
    output::write_output(&mut writer, &[formatted], &args.list.format)?;

    Ok(())
}

async fn run_scheduled(
    args: TestArgs,
    config: &ResolvedConfig,
    db: &Database,
    every: &str,
) -> Result<()> {
    let interval = duration::parse_duration(every)?;
    let interval_display = duration::format_duration(interval);

    // --every implies recording, so provider is required
    let provider = args.provider.or(config.provider).ok_or_else(|| {
        anyhow::anyhow!(
            "--provider is required for scheduled tests (set it in config or pass explicitly)"
        )
    })?;

    info!("running speed tests every {interval_display} (Ctrl+C to stop)");

    let mut iteration = 0u64;
    let mut failures = 0u64;
    let mut ticker = scheduled_ticker(interval);

    // A single long-lived ctrl_c future, awaited across the whole loop rather
    // than only around the wait. Previously Ctrl+C pressed during a test --
    // most of the wall-clock time -- was swallowed, and the process looked
    // unresponsive.
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                info!("stopping scheduled tests ({iteration} tests completed, {failures} failed)");
                break;
            }
            _ = ticker.tick() => {}
        }

        iteration += 1;

        // Errors must not end the loop: a monitor meant to run for weeks
        // should survive a DNS blip or a Wi-Fi drop.
        let outcome = tokio::select! {
            _ = &mut shutdown => {
                info!("stopping scheduled tests ({iteration} tests completed, {failures} failed)");
                break;
            }
            outcome = run_scheduled_iteration(&args, config, db, provider) => outcome,
        };

        match outcome {
            Ok(()) => info!("test #{iteration} complete, next in {interval_display}"),
            Err(e) => {
                failures += 1;
                warn!("test #{iteration} failed ({failures} total), continuing: {e:#}");
            }
        }
    }

    Ok(())
}

/// One scheduled measurement: run, record, render. Returns an error instead of
/// propagating out of the loop, so the caller can log and continue.
async fn run_scheduled_iteration(
    args: &TestArgs,
    config: &ResolvedConfig,
    db: &Database,
    provider: i64,
) -> Result<()> {
    let config_st = bbm::SpeedTestConfig {
        duration_secs: args.resolved_duration(config),
        streams: args.resolved_streams(config),
        peer: args
            .peer
            .clone()
            .or_else(|| config.peer.clone())
            .unwrap_or_else(|| bbm::SpeedTestConfig::DEFAULT_PEER.to_string()),
        ..Default::default()
    };

    let runner = bbm::SpeedTestRunner::new(config_st)?;
    let result = runner.run().await?;

    let m = db
        .measurements()
        .record(
            result.download_kbps,
            result.upload_kbps,
            result.latency_ms,
            Some(provider),
            args.plan.as_deref().or(config.plan.as_deref()),
        )
        .await?;
    info!("recorded measurement #{} at {}", m.id, m.timestamp);

    let formatted = FormattedSpeedTestResult::from_result(&result, &args.unit);
    let mut writer = make_writer(&args.list)?;
    output::write_output(&mut writer, &[formatted], &args.list.format)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Sleeping for the full interval *after* the test finishes makes the true
    /// period `interval + test_duration`, so a 30m schedule slips by the
    /// length of every run -- hours per week for a long-lived monitor. The
    /// period must be measured start-to-start.
    #[tokio::test(start_paused = true)]
    async fn schedule_does_not_drift_with_slow_runs() {
        let period = Duration::from_secs(1800);
        let mut ticker = scheduled_ticker(period);
        let start = tokio::time::Instant::now();

        // First tick fires immediately.
        ticker.tick().await;

        // Simulate a run that takes a meaningful slice of the period.
        tokio::time::sleep(Duration::from_secs(40)).await;

        // Second tick should land one period after the FIRST tick, not one
        // period after the work finished.
        ticker.tick().await;

        let elapsed = start.elapsed();
        assert_eq!(
            elapsed, period,
            "expected start-to-start spacing of {period:?}, got {elapsed:?}"
        );
    }

    /// A run that overruns its own period must not queue up a burst of
    /// immediate catch-up ticks.
    #[tokio::test(start_paused = true)]
    async fn overrunning_run_does_not_burst() {
        let period = Duration::from_secs(60);
        let mut ticker = scheduled_ticker(period);
        let start = tokio::time::Instant::now();

        ticker.tick().await;
        tokio::time::sleep(Duration::from_secs(150)).await; // overruns 2 periods
        ticker.tick().await;
        ticker.tick().await;

        // With Delay behaviour the missed ticks are not replayed back-to-back.
        assert!(
            start.elapsed() >= Duration::from_secs(150) + period,
            "missed ticks should be delayed, not fired immediately: {:?}",
            start.elapsed()
        );
    }
}
