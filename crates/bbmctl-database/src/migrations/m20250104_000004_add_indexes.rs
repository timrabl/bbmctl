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

//! Index the columns every query filters and sorts on.
//!
//! The original schema created no indexes at all, so each of these did a full
//! table scan:
//!
//! - `measurements.timestamp` -- `history`, `export_all`, `history_since`,
//!   `delete_older_than`
//! - `measurements.campaign_id` -- `campaigns().measurements()`,
//!   `measurement_count`, `days_spanned`, `check_timing`, `next_allowed_time`
//! - `campaigns.status` -- `campaigns().active()`, `list()`
//!
//! A measurement history is append-only and grows without bound, so the
//! timestamp scan degrades continuously over the life of the database.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const INDEXES: &[(&str, &str)] = &[
    (
        "idx_measurements_timestamp",
        "CREATE INDEX IF NOT EXISTS idx_measurements_timestamp ON measurements (timestamp)",
    ),
    (
        "idx_measurements_campaign_id",
        "CREATE INDEX IF NOT EXISTS idx_measurements_campaign_id ON measurements (campaign_id)",
    ),
    (
        "idx_campaigns_status",
        "CREATE INDEX IF NOT EXISTS idx_campaigns_status ON campaigns (status)",
    ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for (_, sql) in INDEXES {
            conn.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        for (name, _) in INDEXES {
            conn.execute_unprepared(&format!("DROP INDEX IF EXISTS {name}"))
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Database;
    use sea_orm::{ConnectionTrait, DbBackend, Statement};

    /// Every column the repositories filter or sort on must be indexed.
    #[tokio::test]
    async fn expected_indexes_exist() {
        let db = Database::connect_in_memory().await.unwrap();

        let rows = db
            .conn
            .query_all_raw(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_%'"
                    .to_string(),
            ))
            .await
            .unwrap();

        let names: Vec<String> = rows
            .iter()
            .map(|r| r.try_get_by_index::<String>(0).unwrap())
            .collect();

        for expected in [
            "idx_measurements_timestamp",
            "idx_measurements_campaign_id",
            "idx_campaigns_status",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing index {expected}; found {names:?}"
            );
        }
    }
}
