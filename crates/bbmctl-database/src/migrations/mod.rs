use sea_orm_migration::MigratorTrait;

mod m20250101_000001_create_tables;
mod m20250102_000002_create_settings;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_create_tables::Migration),
            Box::new(m20250102_000002_create_settings::Migration),
        ]
    }
}
