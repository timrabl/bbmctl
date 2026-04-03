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
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

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

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        // Try to find existing
        let existing = settings::Entity::find()
            .filter(settings::Column::Key.eq(key))
            .one(self.conn)
            .await?;

        if let Some(model) = existing {
            let mut active: settings::ActiveModel = model.into();
            active.value = Set(value.to_string());
            active.update(self.conn).await?;
        } else {
            let active = settings::ActiveModel {
                key: Set(key.to_string()),
                value: Set(value.to_string()),
            };
            active.insert(self.conn).await?;
        }

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
