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
    // `compare` already used config.provider for the recording side, so
    // requiring the flag for the lookup was inconsistent with `test` and
    // `history add`, which both accept a configured provider.
    let provider = args.provider.or(config.provider).ok_or_else(|| {
        anyhow::anyhow!(
            "--provider is required (set it in config, or run `bbmctl provider switch`)"
        )
    })?;

    let plans = client.get_plans_by_provider_id(provider).await?;

    if plans.is_empty() {
        anyhow::bail!("no plans found for provider {provider}");
    }

    let target_plans: Vec<&bbm::Plan> = match &args.plan {
        Some(plan_id) => {
            let matched: Vec<_> = plans.iter().filter(|p| p.plan_id.0 == *plan_id).collect();
            if matched.is_empty() {
                anyhow::bail!(
                    "plan '{}' not found for provider {} — use `list plans --provider {}` to see available plans",
                    plan_id, provider, provider
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
