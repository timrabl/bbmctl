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

//! Normalise historical timestamps to a single, sortable representation.
//!
//! Two formats were previously written into `measurements.timestamp`:
//! `%Y-%m-%dT%H:%M:%SZ` for plain records and `to_rfc3339()`
//! (`...+00:00`, with microseconds) for campaign records. The column is a
//! varchar that every query compares lexicographically, and `.` (0x2E) sorts
//! before `Z` (0x5A), so a campaign row written later in the same second
//! sorted *before* a plain one. That made "newest first" wrong and could make
//! `history purge --older-than` delete the wrong rows.
//!
//! This rewrites every existing row to `%Y-%m-%dT%H:%M:%S.mmmZ`. Rows that
//! SQLite cannot parse are left untouched rather than destroyed, so an
//! operator can still find and inspect them.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Rewrite to `YYYY-MM-DDTHH:MM:SS.mmmZ`.
///
/// `strftime` returns NULL for anything it cannot parse; the `WHERE` clause
/// skips those rows so unparseable data is preserved rather than nulled out.
const NORMALIZE: &str = r#"
    UPDATE measurements
    SET timestamp = strftime('%Y-%m-%dT%H:%M:%f', timestamp) || 'Z'
    WHERE strftime('%Y-%m-%dT%H:%M:%f', timestamp) IS NOT NULL
      AND timestamp <> strftime('%Y-%m-%dT%H:%M:%f', timestamp) || 'Z'
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(NORMALIZE)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // Not reversible: the original representation is not recoverable, and
        // the normalised form is a strict improvement.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::NORMALIZE;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    async fn scalar(conn: &sea_orm::DatabaseConnection, sql: &str) -> Vec<String> {
        let rows = conn
            .query_all_raw(Statement::from_string(DbBackend::Sqlite, sql.to_string()))
            .await
            .unwrap();
        rows.iter()
            .map(|r| r.try_get_by_index::<String>(0).unwrap())
            .collect()
    }

    /// The two legacy formats must collapse to one, and ordering must then
    /// match chronology.
    #[tokio::test]
    async fn normalizes_both_legacy_formats_and_fixes_ordering() {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        conn.execute_unprepared(
            "CREATE TABLE measurements (id INTEGER PRIMARY KEY, timestamp VARCHAR NOT NULL)",
        )
        .await
        .unwrap();

        // The campaign row is chronologically LATER, but sorts earlier under
        // the old mixed formats because '.' < 'Z'.
        conn.execute_unprepared(
            "INSERT INTO measurements (id, timestamp) VALUES
                (1, '2026-07-20T09:56:55.672667+00:00'),
                (2, '2026-07-20T09:56:55Z')",
        )
        .await
        .unwrap();

        let before = scalar(
            &conn,
            "SELECT timestamp FROM measurements ORDER BY timestamp",
        )
        .await;
        assert_eq!(
            before[0], "2026-07-20T09:56:55.672667+00:00",
            "precondition: the later row sorts first before normalisation"
        );

        conn.execute_unprepared(NORMALIZE).await.unwrap();

        let after = scalar(
            &conn,
            "SELECT timestamp FROM measurements ORDER BY timestamp",
        )
        .await;
        assert_eq!(
            after,
            vec![
                "2026-07-20T09:56:55.000Z".to_string(),
                // SQLite rounds sub-second precision rather than truncating,
                // so .672667 becomes .673.
                "2026-07-20T09:56:55.673Z".to_string(),
            ],
            "both rows should share one format and sort chronologically"
        );
    }

    /// An unparseable value must survive so it can still be found and removed.
    #[tokio::test]
    async fn leaves_unparseable_timestamps_intact() {
        let conn = Database::connect("sqlite::memory:").await.unwrap();
        conn.execute_unprepared(
            "CREATE TABLE measurements (id INTEGER PRIMARY KEY, timestamp VARCHAR NOT NULL)",
        )
        .await
        .unwrap();
        conn.execute_unprepared(
            "INSERT INTO measurements (id, timestamp) VALUES (1, 'not-a-date')",
        )
        .await
        .unwrap();

        conn.execute_unprepared(NORMALIZE).await.unwrap();

        let after = scalar(&conn, "SELECT timestamp FROM measurements").await;
        assert_eq!(after, vec!["not-a-date".to_string()]);
    }
}
