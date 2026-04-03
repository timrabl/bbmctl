use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "measurements")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub timestamp: String,
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub latency_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip)]
    pub campaign_id: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::campaign::Entity",
        from = "Column::CampaignId",
        to = "super::campaign::Column::Id"
    )]
    Campaign,
}

impl Related<super::campaign::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Campaign.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
