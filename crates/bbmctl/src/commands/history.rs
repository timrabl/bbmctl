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
use crate::cli::HistoryCommands;
use crate::config::ResolvedConfig;
use crate::utils::output;
use bbmctl_database::Database;

pub async fn run(command: HistoryCommands, config: &ResolvedConfig, db: &Database) -> Result<()> {
    match command {
        HistoryCommands::Add(args) => {
            let provider = args.provider.or(config.provider).ok_or_else(|| {
                anyhow::anyhow!("--provider is required (set it in config or pass explicitly)")
            })?;
            let m = db
                .measurements()
                .record(
                    args.download,
                    args.upload,
                    args.latency,
                    Some(provider),
                    args.plan.as_deref().or(config.plan.as_deref()),
                )
                .await?;
            info!("recorded measurement #{} at {}", m.id, m.timestamp);
        }
        HistoryCommands::List(args) => {
            let measurements = db.measurements().history(Some(args.limit as u64)).await?;
            let mut writer = make_writer(&args.list)?;
            output::write_output(&mut writer, &measurements, &args.list.format)?;
        }
        HistoryCommands::Summary(args) => match db.measurements().summary().await? {
            Some(summary) => {
                let mut writer = make_writer(&args.list)?;
                output::write_output(&mut writer, &[summary], &args.list.format)?;
            }
            None => {
                info!("no measurements recorded yet");
            }
        },
        HistoryCommands::Delete(args) => {
            if args.all {
                if !args.confirm {
                    anyhow::bail!("use --confirm to delete all measurements");
                }
                let count = db.measurements().delete_all().await?;
                info!("deleted {count} measurements");
            } else if let Some(id) = args.id {
                let deleted = db.measurements().delete(id).await?;
                if deleted {
                    info!("deleted measurement #{id}");
                } else {
                    anyhow::bail!("measurement #{id} not found");
                }
            } else {
                anyhow::bail!("specify a measurement ID or use --all --confirm");
            }
        }
        HistoryCommands::Purge(args) => {
            if !args.confirm {
                anyhow::bail!("use --confirm to purge measurements");
            }
            let dur = crate::utils::duration::parse_duration(&args.older_than)?;
            let cutoff = chrono::Utc::now() - chrono::Duration::from_std(dur)?;
            let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let count = db.measurements().delete_older_than(&cutoff_str).await?;
            info!("purged {count} measurements older than {}", args.older_than);
        }
        HistoryCommands::Export(args) => {
            let measurements = db.measurements().export_all().await?;
            let writer: Box<dyn std::io::Write> = match &args.output {
                Some(path) => Box::new(std::fs::File::create(path)?),
                None => Box::new(std::io::stdout().lock()),
            };
            let mut wtr = csv::Writer::from_writer(writer);
            wtr.write_record([
                "timestamp",
                "download_kbps",
                "upload_kbps",
                "latency_ms",
                "provider_id",
                "plan_id",
            ])?;
            for m in &measurements {
                wtr.write_record([
                    &m.timestamp,
                    &m.download_kbps.to_string(),
                    &m.upload_kbps.to_string(),
                    &m.latency_ms.to_string(),
                    &m.provider_id.map(|v| v.to_string()).unwrap_or_default(),
                    &m.plan_id.clone().unwrap_or_default(),
                ])?;
            }
            wtr.flush()?;
            info!("exported {} measurements", measurements.len());
        }
        HistoryCommands::Import(args) => {
            let mut rdr = csv::Reader::from_path(&args.file)?;
            let mut count = 0u64;
            for result in rdr.records() {
                let record = result?;
                let timestamp = record.get(0).unwrap_or_default();
                let download_kbps: f64 = record.get(1).unwrap_or("0").parse()?;
                let upload_kbps: f64 = record.get(2).unwrap_or("0").parse()?;
                let latency_ms: f64 = record.get(3).unwrap_or("0").parse()?;
                let provider_id: Option<i64> =
                    record
                        .get(4)
                        .and_then(|s| if s.is_empty() { None } else { s.parse().ok() });
                let plan_id: Option<&str> = record.get(5).filter(|s| !s.is_empty());

                db.measurements()
                    .record_with_timestamp(
                        timestamp,
                        download_kbps,
                        upload_kbps,
                        latency_ms,
                        provider_id,
                        plan_id,
                    )
                    .await?;
                count += 1;
            }
            info!("imported {count} measurements");
        }
        HistoryCommands::Trend(args) => {
            let measurements = if let Some(ref last) = args.last {
                let dur = crate::utils::duration::parse_duration(last)?;
                let since = chrono::Utc::now() - chrono::Duration::from_std(dur)?;
                let since_str = since.format("%Y-%m-%dT%H:%M:%SZ").to_string();
                db.measurements().history_since(&since_str).await?
            } else {
                db.measurements().history(Some(args.limit as u64)).await?
            };

            if measurements.is_empty() {
                info!("no measurements found");
                return Ok(());
            }

            // Measurements come in DESC order, reverse for chronological sparkline
            let mut measurements = measurements;
            measurements.reverse();

            let download_kbps: Vec<f64> = measurements.iter().map(|m| m.download_kbps).collect();
            let upload_kbps: Vec<f64> = measurements.iter().map(|m| m.upload_kbps).collect();
            let latency_ms: Vec<f64> = measurements.iter().map(|m| m.latency_ms).collect();

            let output = crate::utils::output::render_trend(
                &download_kbps,
                &upload_kbps,
                &latency_ms,
                &args.unit,
            );
            print!("{output}");
        }
    }

    Ok(())
}
