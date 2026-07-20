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
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryOrder, QuerySelect,
    Set,
};
use serde::Serialize;

use crate::entities::measurement;

#[derive(Debug, Clone, Serialize)]
pub struct MeasurementSummary {
    pub count: u64,
    pub first: String,
    pub last: String,
    pub days_spanned: u64,
    pub avg_download_kbps: f64,
    pub avg_upload_kbps: f64,
    pub avg_latency_ms: f64,
    pub min_download_kbps: f64,
    pub max_download_kbps: f64,
    pub min_upload_kbps: f64,
    pub max_upload_kbps: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
}

pub struct MeasurementRepo<'a> {
    conn: &'a DatabaseConnection,
}

impl<'a> MeasurementRepo<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn record(
        &self,
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
        provider_id: Option<i64>,
        plan_id: Option<&str>,
    ) -> Result<measurement::Model> {
        let now = crate::now_timestamp();

        let active = measurement::ActiveModel {
            timestamp: Set(now),
            download_kbps: Set(download_kbps),
            upload_kbps: Set(upload_kbps),
            latency_ms: Set(latency_ms),
            provider_id: Set(provider_id),
            plan_id: Set(plan_id.map(|s| s.to_string())),
            ..Default::default()
        };

        let model = active.insert(self.conn).await?;
        Ok(model)
    }

    pub async fn history(&self, limit: Option<u64>) -> Result<Vec<measurement::Model>> {
        let limit = limit.unwrap_or(100);
        let models = measurement::Entity::find()
            .order_by_desc(measurement::Column::Timestamp)
            .limit(limit)
            .all(self.conn)
            .await?;
        Ok(models)
    }

    pub async fn history_since(&self, since: &str) -> Result<Vec<measurement::Model>> {
        use sea_orm::{ColumnTrait, QueryFilter};
        let models = measurement::Entity::find()
            .filter(measurement::Column::Timestamp.gte(since.to_string()))
            .order_by_desc(measurement::Column::Timestamp)
            .all(self.conn)
            .await?;
        Ok(models)
    }

    pub async fn export_all(&self) -> Result<Vec<measurement::Model>> {
        let models = measurement::Entity::find()
            .order_by_asc(measurement::Column::Timestamp)
            .all(self.conn)
            .await?;
        Ok(models)
    }

    pub async fn record_with_timestamp(
        &self,
        timestamp: &str,
        download_kbps: f64,
        upload_kbps: f64,
        latency_ms: f64,
        provider_id: Option<i64>,
        plan_id: Option<&str>,
    ) -> Result<measurement::Model> {
        let active = measurement::ActiveModel {
            timestamp: Set(timestamp.to_string()),
            download_kbps: Set(download_kbps),
            upload_kbps: Set(upload_kbps),
            latency_ms: Set(latency_ms),
            provider_id: Set(provider_id),
            plan_id: Set(plan_id.map(|s| s.to_string())),
            ..Default::default()
        };
        let model = active.insert(self.conn).await?;
        Ok(model)
    }

    pub async fn delete(&self, id: i64) -> Result<bool> {
        let result = measurement::Entity::delete_by_id(id)
            .exec(self.conn)
            .await?;
        Ok(result.rows_affected > 0)
    }

    pub async fn delete_older_than(&self, before: &str) -> Result<u64> {
        use sea_orm::{ColumnTrait, QueryFilter};
        let result = measurement::Entity::delete_many()
            .filter(measurement::Column::Timestamp.lt(before.to_string()))
            .exec(self.conn)
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn delete_all(&self) -> Result<u64> {
        let result = measurement::Entity::delete_many().exec(self.conn).await?;
        Ok(result.rows_affected)
    }

    pub async fn summary(&self) -> Result<Option<MeasurementSummary>> {
        let sql = r#"
            SELECT
                COUNT(*) as count,
                MIN(timestamp) as first,
                MAX(timestamp) as last,
                CAST(ROUND(JULIANDAY(MAX(timestamp)) - JULIANDAY(MIN(timestamp))) AS INTEGER) as days_spanned,
                AVG(download_kbps) as avg_download_kbps,
                AVG(upload_kbps) as avg_upload_kbps,
                AVG(latency_ms) as avg_latency_ms,
                MIN(download_kbps) as min_download_kbps,
                MAX(download_kbps) as max_download_kbps,
                MIN(upload_kbps) as min_upload_kbps,
                MAX(upload_kbps) as max_upload_kbps,
                MIN(latency_ms) as min_latency_ms,
                MAX(latency_ms) as max_latency_ms
            FROM measurements
        "#;

        let statement =
            sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql.to_string());
        let row = self.conn.query_one_raw(statement).await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let count: i64 = row.try_get_by_index(0)?;
        if count == 0 {
            return Ok(None);
        }

        let first: String = row.try_get_by_index(1)?;
        let last: String = row.try_get_by_index(2)?;
        // JULIANDAY yields NULL for a timestamp SQLite cannot parse, so this
        // must be nullable: otherwise one malformed row breaks the entire
        // summary (and the Prometheus exporter) permanently.
        let days_spanned: i32 = row.try_get_by_index::<Option<i32>>(3)?.unwrap_or(0);
        let avg_download_kbps: f64 = row.try_get_by_index(4)?;
        let avg_upload_kbps: f64 = row.try_get_by_index(5)?;
        let avg_latency_ms: f64 = row.try_get_by_index(6)?;
        let min_download_kbps: f64 = row.try_get_by_index(7)?;
        let max_download_kbps: f64 = row.try_get_by_index(8)?;
        let min_upload_kbps: f64 = row.try_get_by_index(9)?;
        let max_upload_kbps: f64 = row.try_get_by_index(10)?;
        let min_latency_ms: f64 = row.try_get_by_index(11)?;
        let max_latency_ms: f64 = row.try_get_by_index(12)?;

        Ok(Some(MeasurementSummary {
            count: count as u64,
            first,
            last,
            days_spanned: days_spanned as u64,
            avg_download_kbps,
            avg_upload_kbps,
            avg_latency_ms,
            min_download_kbps,
            max_download_kbps,
            min_upload_kbps,
            max_upload_kbps,
            min_latency_ms,
            max_latency_ms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;

    /// `measurements.timestamp` is a varchar compared lexicographically by
    /// every query. Plain records used `%Y-%m-%dT%H:%M:%SZ` while campaign
    /// records used `to_rfc3339()` (`+00:00`, with microseconds). Since '.'
    /// (0x2E) sorts before 'Z' (0x5A), a campaign row written later in the
    /// same second sorted *before* a plain one -- so "newest first" lied, and
    /// `history purge --older-than` could delete the wrong rows.
    #[tokio::test]
    async fn all_writers_use_one_timestamp_format() {
        let db = Database::connect_in_memory().await.unwrap();

        let plain = db
            .measurements()
            .record(100_000.0, 50_000.0, 10.0, None, None)
            .await
            .unwrap();

        let campaign = db.campaigns().start(1, "plan-1").await.unwrap();
        let via_campaign = db
            .campaigns()
            .record(campaign.id, 200_000.0, 60_000.0, 20.0)
            .await
            .unwrap();

        // Both rows must be directly comparable as strings.
        assert_eq!(
            plain.timestamp.len(),
            via_campaign.timestamp.len(),
            "timestamps have different shapes: {:?} vs {:?}",
            plain.timestamp,
            via_campaign.timestamp
        );
        assert!(
            plain.timestamp.ends_with('Z') && via_campaign.timestamp.ends_with('Z'),
            "both must use the same UTC suffix: {:?} vs {:?}",
            plain.timestamp,
            via_campaign.timestamp
        );
    }

    /// `JULIANDAY` returns NULL for a timestamp it cannot parse, and decoding
    /// that as a non-null i32 fails. A single bad row -- reachable through
    /// `history import`, which does no validation -- therefore breaks
    /// `history summary` and the Prometheus exporter permanently, with no CLI
    /// path to find or remove the offending row.
    #[tokio::test]
    async fn summary_survives_an_unparseable_timestamp() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record(100_000.0, 50_000.0, 10.0, None, None)
            .await
            .unwrap();
        repo.record_with_timestamp("not-a-date", 200_000.0, 60_000.0, 20.0, None, None)
            .await
            .unwrap();

        let summary = repo
            .summary()
            .await
            .expect("one malformed row must not break the whole summary")
            .expect("summary should exist for a non-empty table");

        assert_eq!(summary.count, 2);
    }

    #[tokio::test]
    async fn record_and_history() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        let m1 = repo
            .record(100_000.0, 50_000.0, 10.0, None, None)
            .await
            .unwrap();
        assert!((m1.download_kbps - 100_000.0).abs() < f64::EPSILON);
        assert!(m1.provider_id.is_none());

        let m2 = repo
            .record(200_000.0, 80_000.0, 15.0, Some(1), Some("basic"))
            .await
            .unwrap();
        assert!((m2.download_kbps - 200_000.0).abs() < f64::EPSILON);
        assert_eq!(m2.provider_id, Some(1));
        assert_eq!(m2.plan_id.as_deref(), Some("basic"));

        let history = repo.history(None).await.unwrap();
        assert_eq!(history.len(), 2);
        // Both records present, ordered by timestamp DESC (same second means order by rowid)
        let downloads: Vec<f64> = history.iter().map(|m| m.download_kbps).collect();
        assert!(downloads.contains(&100_000.0));
        assert!(downloads.contains(&200_000.0));
    }

    #[tokio::test]
    async fn summary_empty() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        let summary = repo.summary().await.unwrap();
        assert!(summary.is_none());
    }

    #[tokio::test]
    async fn summary_with_data() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record(100_000.0, 40_000.0, 10.0, None, None)
            .await
            .unwrap();
        repo.record(200_000.0, 60_000.0, 20.0, None, None)
            .await
            .unwrap();

        let summary = repo.summary().await.unwrap().unwrap();
        assert_eq!(summary.count, 2);
        assert!((summary.avg_download_kbps - 150_000.0).abs() < f64::EPSILON);
        assert!((summary.avg_upload_kbps - 50_000.0).abs() < f64::EPSILON);
        assert!((summary.avg_latency_ms - 15.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn history_limit() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        for i in 0..10 {
            repo.record(i as f64 * 1000.0, 1000.0, 5.0, None, None)
                .await
                .unwrap();
        }

        let history = repo.history(Some(3)).await.unwrap();
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn delete_by_id() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        let model = repo
            .record(100_000.0, 50_000.0, 10.0, None, None)
            .await
            .unwrap();

        let deleted = repo.delete(model.id).await.unwrap();
        assert!(deleted);

        let deleted_again = repo.delete(model.id).await.unwrap();
        assert!(!deleted_again);

        let history = repo.history(None).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn delete_all() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record(100_000.0, 50_000.0, 10.0, None, None)
            .await
            .unwrap();
        repo.record(200_000.0, 80_000.0, 15.0, None, None)
            .await
            .unwrap();

        let count = repo.delete_all().await.unwrap();
        assert_eq!(count, 2);

        let history = repo.history(None).await.unwrap();
        assert!(history.is_empty());
    }
}
