use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "campaigns")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub provider_id: i64,
    pub plan_id: String,
    pub started_at: String,
    #[sea_orm(default_value = "active")]
    pub status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::measurement::Entity")]
    Measurements,
}

impl Related<super::measurement::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Measurements.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
