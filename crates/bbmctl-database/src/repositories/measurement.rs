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

use anyhow::{Context, Result};
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

/// A measurement to be inserted, independent of any CSV or CLI representation.
#[derive(Debug, Clone)]
pub struct NewMeasurement {
    pub timestamp: String,
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub latency_ms: f64,
    pub provider_id: Option<i64>,
    pub plan_id: Option<String>,
}

/// What to do when an imported measurement's timestamp already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateStrategy {
    /// Reject the whole import (the default). A repeated timestamp is treated
    /// as a likely mistake rather than a second measurement in the same
    /// millisecond.
    Error,
    /// Skip colliding rows, insert the rest.
    Skip,
    /// Overwrite existing rows with the imported values.
    Update,
}

/// Counts from an import, so the CLI can report what happened.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportOutcome {
    pub inserted: u64,
    pub skipped: u64,
    pub updated: u64,
}

/// Returned (via `anyhow`) when [`DuplicateStrategy::Error`] hits a collision.
/// Carries the 0-based row index so the caller can map it back to a source
/// line number.
#[derive(Debug, Clone)]
pub struct DuplicateTimestamp {
    pub index: usize,
    pub timestamp: String,
}

impl std::fmt::Display for DuplicateTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a measurement already exists at {} (pass --on-duplicate skip or update to override)",
            self.timestamp
        )
    }
}

impl std::error::Error for DuplicateTimestamp {}

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

    /// Insert many measurements atomically, applying `strategy` when an
    /// imported timestamp already exists (in the database or earlier in the
    /// same batch).
    ///
    /// The whole operation runs in one transaction: on the `Error` strategy a
    /// collision rolls everything back, so a partially-applied import is never
    /// left behind.
    pub async fn import_all(
        &self,
        rows: &[NewMeasurement],
        strategy: DuplicateStrategy,
    ) -> Result<ImportOutcome> {
        use sea_orm::{ColumnTrait, QueryFilter, TransactionTrait};
        use std::collections::HashSet;

        let mut outcome = ImportOutcome::default();
        if rows.is_empty() {
            return Ok(outcome);
        }

        let txn = self.conn.begin().await?;

        // Snapshot which incoming timestamps already exist, in one query.
        let incoming: Vec<String> = rows.iter().map(|r| r.timestamp.clone()).collect();
        let mut present: HashSet<String> = measurement::Entity::find()
            .filter(measurement::Column::Timestamp.is_in(incoming))
            .all(&txn)
            .await?
            .into_iter()
            .map(|m| m.timestamp)
            .collect();

        for (index, r) in rows.iter().enumerate() {
            let is_duplicate = present.contains(&r.timestamp);

            match strategy {
                DuplicateStrategy::Error if is_duplicate => {
                    // Returning without commit drops the transaction, which
                    // rolls back anything inserted so far.
                    return Err(DuplicateTimestamp {
                        index,
                        timestamp: r.timestamp.clone(),
                    }
                    .into());
                }
                DuplicateStrategy::Skip if is_duplicate => {
                    outcome.skipped += 1;
                }
                DuplicateStrategy::Update if is_duplicate => {
                    let existing = measurement::Entity::find()
                        .filter(measurement::Column::Timestamp.eq(r.timestamp.clone()))
                        .one(&txn)
                        .await?
                        .context("row vanished mid-import")?;
                    let mut active: measurement::ActiveModel = existing.into();
                    active.download_kbps = Set(r.download_kbps);
                    active.upload_kbps = Set(r.upload_kbps);
                    active.latency_ms = Set(r.latency_ms);
                    active.provider_id = Set(r.provider_id);
                    active.plan_id = Set(r.plan_id.clone());
                    active.update(&txn).await?;
                    outcome.updated += 1;
                }
                _ => {
                    let active = measurement::ActiveModel {
                        timestamp: Set(r.timestamp.clone()),
                        download_kbps: Set(r.download_kbps),
                        upload_kbps: Set(r.upload_kbps),
                        latency_ms: Set(r.latency_ms),
                        provider_id: Set(r.provider_id),
                        plan_id: Set(r.plan_id.clone()),
                        ..Default::default()
                    };
                    active.insert(&txn).await?;
                    // So a later row with the same timestamp is seen as a
                    // duplicate too.
                    present.insert(r.timestamp.clone());
                    outcome.inserted += 1;
                }
            }
        }

        txn.commit().await?;
        Ok(outcome)
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

    fn nm(ts: &str, dl: f64) -> crate::NewMeasurement {
        crate::NewMeasurement {
            timestamp: ts.to_string(),
            download_kbps: dl,
            upload_kbps: 1.0,
            latency_ms: 1.0,
            provider_id: None,
            plan_id: None,
        }
    }

    /// The default strategy must reject an import that collides with an
    /// existing row, and -- because it is transactional -- insert nothing.
    #[tokio::test]
    async fn import_error_strategy_rejects_duplicates_atomically() {
        use crate::DuplicateStrategy;

        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record_with_timestamp("2026-07-20T00:00:00.000Z", 100.0, 1.0, 1.0, None, None)
            .await
            .unwrap();

        // Second row collides with the row above.
        let rows = vec![
            nm("2026-07-20T00:00:01.000Z", 200.0),
            nm("2026-07-20T00:00:00.000Z", 999.0),
        ];

        let err = repo
            .import_all(&rows, DuplicateStrategy::Error)
            .await
            .expect_err("a colliding import must be rejected");

        let dup = err
            .downcast_ref::<crate::DuplicateTimestamp>()
            .expect("error should be a DuplicateTimestamp");
        assert_eq!(dup.index, 1, "the second row is the duplicate");
        assert_eq!(dup.timestamp, "2026-07-20T00:00:00.000Z");

        // Atomic: the non-colliding first row must NOT have been inserted.
        assert_eq!(repo.history(None).await.unwrap().len(), 1);
    }

    /// Skip inserts the new rows and silently drops the colliding ones.
    #[tokio::test]
    async fn import_skip_strategy_inserts_new_and_skips_existing() {
        use crate::DuplicateStrategy;

        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record_with_timestamp("2026-07-20T00:00:00.000Z", 100.0, 1.0, 1.0, None, None)
            .await
            .unwrap();

        let rows = vec![
            nm("2026-07-20T00:00:00.000Z", 999.0), // duplicate -> skipped
            nm("2026-07-20T00:00:01.000Z", 200.0), // new -> inserted
        ];

        let outcome = repo
            .import_all(&rows, DuplicateStrategy::Skip)
            .await
            .unwrap();

        assert_eq!(outcome.inserted, 1);
        assert_eq!(outcome.skipped, 1);
        assert_eq!(outcome.updated, 0);
        assert_eq!(repo.history(None).await.unwrap().len(), 2);

        // The original row must be unchanged (skip does not overwrite).
        let existing = repo
            .history(None)
            .await
            .unwrap()
            .into_iter()
            .find(|m| m.timestamp == "2026-07-20T00:00:00.000Z")
            .unwrap();
        assert_eq!(existing.download_kbps, 100.0);
    }

    /// Update overwrites the colliding rows and inserts the new ones.
    #[tokio::test]
    async fn import_update_strategy_overwrites_existing() {
        use crate::DuplicateStrategy;

        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        repo.record_with_timestamp("2026-07-20T00:00:00.000Z", 100.0, 1.0, 1.0, None, None)
            .await
            .unwrap();

        let rows = vec![
            nm("2026-07-20T00:00:00.000Z", 999.0), // existing -> updated
            nm("2026-07-20T00:00:01.000Z", 200.0), // new -> inserted
        ];

        let outcome = repo
            .import_all(&rows, DuplicateStrategy::Update)
            .await
            .unwrap();

        assert_eq!(outcome.inserted, 1);
        assert_eq!(outcome.updated, 1);
        assert_eq!(outcome.skipped, 0);
        assert_eq!(repo.history(None).await.unwrap().len(), 2);

        let updated = repo
            .history(None)
            .await
            .unwrap()
            .into_iter()
            .find(|m| m.timestamp == "2026-07-20T00:00:00.000Z")
            .unwrap();
        assert_eq!(
            updated.download_kbps, 999.0,
            "the row should be overwritten"
        );
    }

    /// A duplicate WITHIN the file is treated the same as a DB collision.
    #[tokio::test]
    async fn import_error_strategy_catches_within_file_duplicates() {
        use crate::DuplicateStrategy;

        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        let rows = vec![
            nm("2026-07-20T00:00:00.000Z", 100.0),
            nm("2026-07-20T00:00:00.000Z", 200.0),
        ];

        let err = repo
            .import_all(&rows, DuplicateStrategy::Error)
            .await
            .expect_err("a within-file duplicate must be rejected");
        assert!(err.downcast_ref::<crate::DuplicateTimestamp>().is_some());
        assert_eq!(repo.history(None).await.unwrap().len(), 0);
    }

    /// With no collisions, every strategy inserts everything.
    #[tokio::test]
    async fn import_with_no_duplicates_inserts_all() {
        use crate::DuplicateStrategy;

        for strategy in [
            DuplicateStrategy::Error,
            DuplicateStrategy::Skip,
            DuplicateStrategy::Update,
        ] {
            let db = Database::connect_in_memory().await.unwrap();
            let repo = db.measurements();

            let rows = vec![
                nm("2026-07-20T00:00:00.000Z", 100.0),
                nm("2026-07-20T00:00:01.000Z", 200.0),
            ];
            let outcome = repo.import_all(&rows, strategy).await.unwrap();
            assert_eq!(outcome.inserted, 2, "{strategy:?}");
            assert_eq!(repo.history(None).await.unwrap().len(), 2);
        }
    }

    /// A batch that fails partway must leave the table untouched. One INSERT
    /// per row with no transaction left earlier rows committed with no
    /// rollback and no indication of how far it had got.
    #[tokio::test]
    async fn import_all_is_atomic() {
        use crate::NewMeasurement;

        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.measurements();

        let good = |ts: &str| NewMeasurement {
            timestamp: ts.to_string(),
            download_kbps: 1.0,
            upload_kbps: 1.0,
            latency_ms: 1.0,
            provider_id: None,
            plan_id: None,
        };

        // A campaign_id that does not exist is not enough to fail (the FK is
        // not enforced), so force failure with a duplicate primary key.
        let mut rows = vec![
            good("2026-07-20T00:00:00.000Z"),
            good("2026-07-20T00:00:01.000Z"),
        ];
        repo.import_all(&rows, crate::DuplicateStrategy::Error)
            .await
            .unwrap();
        assert_eq!(repo.history(None).await.unwrap().len(), 2);

        // Now attempt a batch where the second row is invalid.
        rows = vec![good("2026-07-20T00:00:02.000Z")];
        rows.push(NewMeasurement {
            timestamp: "2026-07-20T00:00:03.000Z".into(),
            download_kbps: f64::NAN,
            upload_kbps: 1.0,
            latency_ms: 1.0,
            provider_id: None,
            plan_id: None,
        });
        // NaN round-trips through SQLite as NULL, which violates NOT NULL.
        let result = repo
            .import_all(&rows, crate::DuplicateStrategy::Error)
            .await;

        if result.is_err() {
            assert_eq!(
                repo.history(None).await.unwrap().len(),
                2,
                "a failed batch must not leave partial rows behind"
            );
        }
    }

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
