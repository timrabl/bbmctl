use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Campaigns::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Campaigns::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Campaigns::ProviderId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Campaigns::PlanId).string().not_null())
                    .col(ColumnDef::new(Campaigns::StartedAt).string().not_null())
                    .col(
                        ColumnDef::new(Campaigns::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Measurements::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Measurements::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Measurements::Timestamp).string().not_null())
                    .col(
                        ColumnDef::new(Measurements::DownloadKbps)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Measurements::UploadKbps).double().not_null())
                    .col(ColumnDef::new(Measurements::LatencyMs).double().not_null())
                    .col(
                        ColumnDef::new(Measurements::ProviderId)
                            .big_integer()
                            .null(),
                    )
                    .col(ColumnDef::new(Measurements::PlanId).string().null())
                    .col(
                        ColumnDef::new(Measurements::CampaignId)
                            .big_integer()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_measurements_campaign_id")
                            .from(Measurements::Table, Measurements::CampaignId)
                            .to(Campaigns::Table, Campaigns::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Measurements::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Campaigns::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Campaigns {
    Table,
    Id,
    ProviderId,
    PlanId,
    StartedAt,
    Status,
}

#[derive(DeriveIden)]
enum Measurements {
    Table,
    Id,
    Timestamp,
    DownloadKbps,
    UploadKbps,
    LatencyMs,
    ProviderId,
    PlanId,
    CampaignId,
}
