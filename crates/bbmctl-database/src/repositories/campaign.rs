use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use log::{info, warn};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseConnection,
    EntityTrait, QueryFilter, QueryOrder, Set, Statement,
};
use serde::Serialize;

use crate::entities::{campaign, measurement};

const CAMPAIGN_REQUIRED_MEASUREMENTS: u64 = 30;
const CAMPAIGN_REQUIRED_DAYS: u64 = 3;
const CAMPAIGN_WINDOW_DAYS: i64 = 14;
const MIN_GAP_SECS: i64 = 300;
const BLOCK_GAP_SECS: i64 = 10_800;
const BLOCK_SIZE: usize = 5;

#[derive(Debug, Clone, Serialize)]
pub struct CampaignStatus {
    pub campaign_id: i64,
    pub provider_id: i64,
    pub plan_id: String,
    pub status: String,
    pub started_at: String,
    pub measurements_total: u64,
    pub measurements_needed: u64,
    pub days_spanned: u64,
    pub days_needed: u64,
    pub days_remaining_in_window: i64,
    #[serde(skip)]
    pub next_measurement_allowed_at: Option<String>,
    #[serde(skip)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignReport {
    pub campaign_id: i64,
    pub provider_id: i64,
    pub plan_id: String,
    pub status: String,
    pub started_at: String,
    pub measurements_total: u64,
    pub days_spanned: u64,
    pub complete: bool,
    pub day_summaries: Vec<CampaignDaySummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignDaySummary {
    pub date: String,
    pub measurements: u64,
    pub avg_download_kbps: f64,
    pub avg_upload_kbps: f64,
    pub avg_latency_ms: f64,
}

pub struct CampaignRepo<'a> {
    conn: &'a DatabaseConnection,
}

impl<'a> CampaignRepo<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn start(&self, provider_id: i64, plan_id: &str) -> Result<campaign::Model> {
        let active = campaign::Entity::find()
            .filter(campaign::Column::Status.eq("active"))
            .one(self.conn)
            .await?;

        if let Some(c) = active {
            bail!(
                "campaign #{} is already active — finish or cancel it first",
                c.id
            );
        }

        let now: DateTime<Utc> = Utc::now();
        let ts = now.to_rfc3339();

        let active_model = campaign::ActiveModel {
            provider_id: Set(provider_id),
            plan_id: Set(plan_id.to_string()),
            started_at: Set(ts),
            status: Set("active".to_string()),
            ..Default::default()
        };

        let model = active_model.insert(self.conn).await?;
        Ok(model)
    }

    pub async fn active(&self) -> Result<Option<campaign::Model>> {
        let model = campaign::Entity::find()
            .filter(campaign::Column::Status.eq("active"))
            .one(self.conn)
            .await?;
        Ok(model)
    }

    pub async fn latest(&self) -> Result<Option<campaign::Model>> {
        let model = campaign::Entity::find()
            .order_by_desc(campaign::Column::StartedAt)
            .one(self.conn)
            .await?;
        Ok(model)
    }

    pub async fn list(&self, status_filter: Option<&str>) -> Result<Vec<campaign::Model>> {
        let mut query = campaign::Entity::find().order_by_desc(campaign::Column::StartedAt);

        if let Some(status) = status_filter {
            query = query.filter(campaign::Column::Status.eq(status));
        }

        let models = query.all(self.conn).await?;
        Ok(models)
    }

    pub async fn record(
        &self,
        campaign_id: i64,
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
    ) -> Result<measurement::Model> {
        let campaign = campaign::Entity::find_by_id(campaign_id)
            .one(self.conn)
            .await?
            .context("campaign not found")?;

        if campaign.status != "active" {
            bail!(
                "campaign #{campaign_id} is not active (status: {})",
                campaign.status
            );
        }

        let started_at = DateTime::parse_from_rfc3339(&campaign.started_at)
            .context("invalid campaign start timestamp")?
            .with_timezone(&Utc);
        let now: DateTime<Utc> = Utc::now();

        let elapsed_days = (now - started_at).num_days();
        if elapsed_days > CAMPAIGN_WINDOW_DAYS {
            let mut update_model: campaign::ActiveModel = campaign.into();
            update_model.status = Set("expired".to_string());
            update_model.update(self.conn).await?;
            bail!(
                "campaign #{campaign_id} has expired — the 14-day window ended on {}",
                (started_at + chrono::Duration::days(CAMPAIGN_WINDOW_DAYS)).format("%Y-%m-%d")
            );
        }

        let warnings = self.check_timing(campaign_id, &now).await?;

        let ts = now.to_rfc3339();
        let active_model = measurement::ActiveModel {
            timestamp: Set(ts),
            download_kbps: Set(download_kbps),
            upload_kbps: Set(upload_kbps),
            latency_ms: Set(latency_ms),
            provider_id: Set(Some(campaign.provider_id)),
            plan_id: Set(Some(campaign.plan_id)),
            campaign_id: Set(Some(campaign_id)),
            ..Default::default()
        };

        let model = active_model.insert(self.conn).await?;

        for w in &warnings {
            warn!("{w}");
        }

        let total = self.measurement_count(campaign_id).await?;
        let days = self.days_spanned(campaign_id).await?;
        if total >= CAMPAIGN_REQUIRED_MEASUREMENTS && days >= CAMPAIGN_REQUIRED_DAYS {
            // Re-fetch the campaign to get the current state for update
            let campaign = campaign::Entity::find_by_id(campaign_id)
                .one(self.conn)
                .await?
                .context("campaign not found")?;
            let mut update_model: campaign::ActiveModel = campaign.into();
            update_model.status = Set("completed".to_string());
            update_model.update(self.conn).await?;
            info!("campaign #{campaign_id} completed! {total} measurements across {days} days.");
        }

        Ok(model)
    }

    pub async fn cancel(&self, campaign_id: i64) -> Result<()> {
        let campaign = campaign::Entity::find_by_id(campaign_id)
            .filter(campaign::Column::Status.eq("active"))
            .one(self.conn)
            .await?;

        match campaign {
            Some(c) => {
                let mut update_model: campaign::ActiveModel = c.into();
                update_model.status = Set("cancelled".to_string());
                update_model.update(self.conn).await?;
                Ok(())
            }
            None => bail!("no active campaign #{campaign_id} to cancel"),
        }
    }

    pub async fn status(&self, campaign_id: i64) -> Result<CampaignStatus> {
        let campaign = campaign::Entity::find_by_id(campaign_id)
            .one(self.conn)
            .await?
            .context("campaign not found")?;

        let total = self.measurement_count(campaign_id).await?;
        let days = self.days_spanned(campaign_id).await?;

        let started_at = DateTime::parse_from_rfc3339(&campaign.started_at)
            .context("invalid campaign start timestamp")?
            .with_timezone(&Utc);
        let now = Utc::now();
        let deadline = started_at + chrono::Duration::days(CAMPAIGN_WINDOW_DAYS);
        let days_remaining = (deadline - now).num_days().max(0);

        let next_allowed = self.next_allowed_time(campaign_id).await?;

        let mut warnings = Vec::new();
        let measurements_needed = CAMPAIGN_REQUIRED_MEASUREMENTS.saturating_sub(total);
        let days_needed = CAMPAIGN_REQUIRED_DAYS.saturating_sub(days);

        if campaign.status == "active" {
            if days_remaining == 0 {
                warnings.push("campaign window has expired!".into());
            } else if days_needed > days_remaining as u64 {
                warnings.push(format!(
                    "need {days_needed} more calendar days but only {days_remaining} days left in window"
                ));
            }
            if measurements_needed > 0 && days_remaining <= 2 {
                warnings.push(format!(
                    "still need {measurements_needed} measurements with only {days_remaining} days left"
                ));
            }
        }

        Ok(CampaignStatus {
            campaign_id: campaign.id,
            provider_id: campaign.provider_id,
            plan_id: campaign.plan_id,
            status: campaign.status,
            started_at: campaign.started_at,
            measurements_total: total,
            measurements_needed,
            days_spanned: days,
            days_needed,
            days_remaining_in_window: days_remaining,
            next_measurement_allowed_at: next_allowed,
            warnings,
        })
    }

    pub async fn measurements(&self, campaign_id: i64) -> Result<Vec<measurement::Model>> {
        let models = measurement::Entity::find()
            .filter(measurement::Column::CampaignId.eq(campaign_id))
            .order_by_asc(measurement::Column::Timestamp)
            .all(self.conn)
            .await?;
        Ok(models)
    }

    pub async fn report(&self, campaign_id: i64) -> Result<CampaignReport> {
        let status = self.status(campaign_id).await?;
        let measurements = self.measurements(campaign_id).await?;

        let mut days: Vec<NaiveDate> = measurements
            .iter()
            .filter_map(|m| {
                DateTime::parse_from_rfc3339(&m.timestamp)
                    .ok()
                    .map(|dt| dt.date_naive())
            })
            .collect();
        days.sort();
        days.dedup();

        let day_summaries: Vec<CampaignDaySummary> = days
            .iter()
            .map(|day| {
                let day_ms: Vec<&measurement::Model> = measurements
                    .iter()
                    .filter(|m| {
                        DateTime::parse_from_rfc3339(&m.timestamp)
                            .ok()
                            .is_some_and(|dt| dt.date_naive() == *day)
                    })
                    .collect();

                let n = day_ms.len() as f64;

                CampaignDaySummary {
                    date: day.to_string(),
                    measurements: day_ms.len() as u64,
                    avg_download_kbps: (day_ms.iter().map(|m| m.download_kbps).sum::<f64>() / n
                        * 100.0)
                        .round()
                        / 100.0,
                    avg_upload_kbps: (day_ms.iter().map(|m| m.upload_kbps).sum::<f64>() / n
                        * 100.0)
                        .round()
                        / 100.0,
                    avg_latency_ms: (day_ms.iter().map(|m| m.latency_ms).sum::<f64>() / n * 100.0)
                        .round()
                        / 100.0,
                }
            })
            .collect();

        let complete = status.status == "completed";

        Ok(CampaignReport {
            campaign_id: status.campaign_id,
            provider_id: status.provider_id,
            plan_id: status.plan_id,
            status: status.status,
            started_at: status.started_at,
            measurements_total: status.measurements_total,
            days_spanned: status.days_spanned,
            complete,
            day_summaries,
        })
    }

    async fn check_timing(&self, campaign_id: i64, now: &DateTime<Utc>) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        let today = now.format("%Y-%m-%d").to_string();

        let sql = "SELECT timestamp FROM measurements WHERE campaign_id = ?1 AND DATE(timestamp) = ?2 ORDER BY timestamp ASC";
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            [campaign_id.into(), today.into()],
        );
        let rows = self.conn.query_all_raw(stmt).await?;

        let mut today_timestamps: Vec<String> = Vec::new();
        for row in &rows {
            let ts: String = row.try_get_by_index(0)?;
            today_timestamps.push(ts);
        }

        let today_count = today_timestamps.len();

        if let Some(last_ts_str) = today_timestamps.last() {
            let last_ts = DateTime::parse_from_rfc3339(last_ts_str)
                .context("invalid measurement timestamp")?
                .with_timezone(&Utc);
            let gap_secs = (*now - last_ts).num_seconds();

            if today_count == BLOCK_SIZE {
                if gap_secs < BLOCK_GAP_SECS {
                    let wait_mins = (BLOCK_GAP_SECS - gap_secs) / 60;
                    warnings.push(format!(
                        "only {gap_secs}s since measurement #{today_count} today — \
                         the protocol requires a 3-hour gap between measurements {BLOCK_SIZE} and {}. \
                         Wait ~{wait_mins} more minutes.",
                        BLOCK_SIZE + 1
                    ));
                }
            } else if gap_secs < MIN_GAP_SECS {
                let wait_secs = MIN_GAP_SECS - gap_secs;
                warnings.push(format!(
                    "only {gap_secs}s since last measurement — \
                     the protocol requires at least 5 minutes between measurements. \
                     Wait {wait_secs} more seconds."
                ));
            }
        }

        Ok(warnings)
    }

    async fn measurement_count(&self, campaign_id: i64) -> Result<u64> {
        let sql = "SELECT COUNT(*) FROM measurements WHERE campaign_id = ?1";
        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, [campaign_id.into()]);
        let row = self
            .conn
            .query_one_raw(stmt)
            .await?
            .context("expected count row")?;
        let count: i64 = row.try_get_by_index(0)?;
        Ok(count as u64)
    }

    async fn days_spanned(&self, campaign_id: i64) -> Result<u64> {
        let sql = "SELECT COUNT(DISTINCT DATE(timestamp)) FROM measurements WHERE campaign_id = ?1";
        let stmt =
            Statement::from_sql_and_values(DatabaseBackend::Sqlite, sql, [campaign_id.into()]);
        let row = self
            .conn
            .query_one_raw(stmt)
            .await?
            .context("expected count row")?;
        let count: i64 = row.try_get_by_index(0)?;
        Ok(count as u64)
    }

    async fn next_allowed_time(&self, campaign_id: i64) -> Result<Option<String>> {
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let sql = "SELECT COUNT(*), MAX(timestamp) FROM measurements WHERE campaign_id = ?1 AND DATE(timestamp) = ?2";
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            sql,
            [campaign_id.into(), today.into()],
        );
        let row = self
            .conn
            .query_one_raw(stmt)
            .await?
            .context("expected row")?;

        let count: i64 = row.try_get_by_index(0)?;
        if count == 0 {
            return Ok(None);
        }

        let last_ts_str: String = row.try_get_by_index(1)?;

        let last_ts = DateTime::parse_from_rfc3339(&last_ts_str)
            .context("invalid timestamp")?
            .with_timezone(&Utc);

        let gap = if count as usize == BLOCK_SIZE {
            chrono::Duration::seconds(BLOCK_GAP_SECS)
        } else {
            chrono::Duration::seconds(MIN_GAP_SECS)
        };

        let next = last_ts + gap;
        if next <= Utc::now() {
            Ok(None)
        } else {
            Ok(Some(next.to_rfc3339()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;

    #[tokio::test]
    async fn start_and_status() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.campaigns();

        let c = repo.start(1, "plan-42").await.unwrap();
        assert_eq!(c.provider_id, 1);
        assert_eq!(c.plan_id, "plan-42");
        assert_eq!(c.status, "active");

        let status = repo.status(c.id).await.unwrap();
        assert_eq!(status.measurements_total, 0);
        assert_eq!(status.measurements_needed, 30);
        assert_eq!(status.days_spanned, 0);
    }

    #[tokio::test]
    async fn no_duplicate_active() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.campaigns();

        repo.start(1, "plan-1").await.unwrap();
        let err = repo.start(2, "plan-2").await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn record_and_report() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.campaigns();

        let c = repo.start(1, "plan-1").await.unwrap();
        repo.record(c.id, 100_000.0, 50_000.0, 10.0).await.unwrap();
        repo.record(c.id, 95_000.0, 48_000.0, 12.0).await.unwrap();

        let status = repo.status(c.id).await.unwrap();
        assert_eq!(status.measurements_total, 2);
        assert_eq!(status.measurements_needed, 28);

        let report = repo.report(c.id).await.unwrap();
        assert_eq!(report.measurements_total, 2);
        assert!(!report.complete);
        assert_eq!(report.day_summaries.len(), 1);
    }

    #[tokio::test]
    async fn cancel() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.campaigns();

        let c = repo.start(1, "plan-1").await.unwrap();
        repo.cancel(c.id).await.unwrap();

        let active = repo.active().await.unwrap();
        assert!(active.is_none());

        let c2 = repo.start(2, "plan-2").await.unwrap();
        assert_eq!(c2.status, "active");
    }

    #[tokio::test]
    async fn measurements_linked() {
        let db = Database::connect_in_memory().await.unwrap();

        let c = db.campaigns().start(1, "plan-1").await.unwrap();
        db.campaigns()
            .record(c.id, 100_000.0, 50_000.0, 10.0)
            .await
            .unwrap();
        db.measurements()
            .record(200_000.0, 100_000.0, 5.0, None, None)
            .await
            .unwrap();

        let campaign_ms = db.campaigns().measurements(c.id).await.unwrap();
        assert_eq!(campaign_ms.len(), 1);

        let all = db.measurements().history(None).await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn list_campaigns() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.campaigns();

        let c1 = repo.start(1, "plan-1").await.unwrap();
        repo.cancel(c1.id).await.unwrap();

        let _c2 = repo.start(2, "plan-2").await.unwrap();

        // All campaigns
        let all = repo.list(None).await.unwrap();
        assert_eq!(all.len(), 2);

        // Only active
        let active = repo.list(Some("active")).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].provider_id, 2);

        // Only cancelled
        let cancelled = repo.list(Some("cancelled")).await.unwrap();
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].provider_id, 1);

        // Non-existent status
        let none = repo.list(Some("completed")).await.unwrap();
        assert!(none.is_empty());
    }
}
