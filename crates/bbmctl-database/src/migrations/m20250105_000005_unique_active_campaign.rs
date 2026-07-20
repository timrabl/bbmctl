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

//! Enforce "at most one active campaign" in the database.
//!
//! `CampaignRepo::start` checks for an existing active campaign and then
//! inserts, with nothing between the two steps. Two concurrent starts both
//! pass the check and both insert; `active()` then silently returns whichever
//! `.one()` happens to pick. Since a campaign backs a BNetzA
//! Nachweisverfahren, the invariant belongs in the database rather than in one
//! code path.
//!
//! A partial index constrains only `status = 'active'`, so any number of
//! completed, expired or cancelled campaigns remain valid.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const CREATE: &str = "CREATE UNIQUE INDEX IF NOT EXISTS idx_campaigns_one_active \
                      ON campaigns (status) WHERE status = 'active'";

/// Leave the newest active campaign active and retire the rest, so the index
/// can be created on a database that already violates the invariant.
const RECONCILE: &str = "UPDATE campaigns SET status = 'expired' \
                         WHERE status = 'active' \
                           AND id <> (SELECT MAX(id) FROM campaigns WHERE status = 'active')";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(RECONCILE).await?;
        conn.execute_unprepared(CREATE).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_campaigns_one_active")
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CREATE, RECONCILE};
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    async fn setup() -> sea_orm::DatabaseConnection {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        conn.execute_unprepared(
            "CREATE TABLE campaigns (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 provider_id INTEGER NOT NULL,
                 plan_id VARCHAR NOT NULL,
                 started_at VARCHAR NOT NULL,
                 status VARCHAR NOT NULL
             )",
        )
        .await
        .unwrap();
        conn
    }

    async fn statuses(conn: &sea_orm::DatabaseConnection) -> Vec<(i32, String)> {
        conn.query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT id, status FROM campaigns ORDER BY id".to_string(),
        ))
        .await
        .unwrap()
        .iter()
        .map(|r| {
            (
                r.try_get_by_index::<i32>(0).unwrap(),
                r.try_get_by_index::<String>(1).unwrap(),
            )
        })
        .collect()
    }

    /// A database that already has several active campaigns must still be
    /// migratable: the newest is kept, the rest are retired.
    #[tokio::test]
    async fn reconciles_preexisting_duplicates_keeping_the_newest() {
        let conn = setup().await;
        conn.execute_unprepared(
            "INSERT INTO campaigns (provider_id, plan_id, started_at, status) VALUES
                 (1, 'a', '2026-01-01T00:00:00.000Z', 'active'),
                 (2, 'b', '2026-01-02T00:00:00.000Z', 'active'),
                 (3, 'c', '2026-01-03T00:00:00.000Z', 'active')",
        )
        .await
        .unwrap();

        conn.execute_unprepared(RECONCILE).await.unwrap();
        conn.execute_unprepared(CREATE)
            .await
            .expect("the index must be creatable after reconciliation");

        assert_eq!(
            statuses(&conn).await,
            vec![
                (1, "expired".to_string()),
                (2, "expired".to_string()),
                (3, "active".to_string()),
            ]
        );
    }

    /// A single active campaign is untouched.
    #[tokio::test]
    async fn leaves_a_single_active_campaign_alone() {
        let conn = setup().await;
        conn.execute_unprepared(
            "INSERT INTO campaigns (provider_id, plan_id, started_at, status)
             VALUES (1, 'a', '2026-01-01T00:00:00.000Z', 'active')",
        )
        .await
        .unwrap();

        conn.execute_unprepared(RECONCILE).await.unwrap();
        conn.execute_unprepared(CREATE).await.unwrap();

        assert_eq!(statuses(&conn).await, vec![(1, "active".to_string())]);
    }
}
