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
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::settings;

pub struct SettingsRepo<'a> {
    conn: &'a DatabaseConnection,
}

impl<'a> SettingsRepo<'a> {
    pub fn new(conn: &'a DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let result = settings::Entity::find()
            .filter(settings::Column::Key.eq(key))
            .one(self.conn)
            .await?;
        Ok(result.map(|m| m.value))
    }

    /// Insert or replace a setting in one statement.
    ///
    /// This was read-then-write, so two concurrent writers could both observe
    /// "absent" and both insert. An upsert makes it a single atomic statement.
    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let active = settings::ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
        };

        settings::Entity::insert(active)
            .on_conflict(
                OnConflict::column(settings::Column::Key)
                    .update_column(settings::Column::Value)
                    .to_owned(),
            )
            .exec(self.conn)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<bool> {
        let result = settings::Entity::delete_many()
            .filter(settings::Column::Key.eq(key))
            .exec(self.conn)
            .await?;
        Ok(result.rows_affected > 0)
    }
}

#[cfg(test)]
mod upsert_tests {
    use crate::Database;

    /// Writing the same key twice must update in place, not fail or duplicate.
    #[tokio::test]
    async fn set_overwrites_an_existing_key() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.settings();

        repo.set("active_provider", "437").await.unwrap();
        repo.set("active_provider", "999").await.unwrap();

        assert_eq!(
            repo.get("active_provider").await.unwrap().as_deref(),
            Some("999")
        );
    }

    /// Repeating the same write is harmless.
    #[tokio::test]
    async fn set_is_idempotent() {
        let db = Database::connect_in_memory().await.unwrap();
        let repo = db.settings();

        for _ in 0..3 {
            repo.set("k", "v").await.unwrap();
        }

        assert_eq!(repo.get("k").await.unwrap().as_deref(), Some("v"));
    }
}
