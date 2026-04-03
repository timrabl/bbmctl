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
