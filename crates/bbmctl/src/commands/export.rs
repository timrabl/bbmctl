use anyhow::Result;

use crate::cli::ExportCommands;
use crate::prometheus;
use bbmctl_database::Database;

pub async fn run(command: ExportCommands, db: &Database) -> Result<()> {
    match command {
        ExportCommands::Prometheus(args) => {
            prometheus::serve(args.port, db).await?;
        }
    }
    Ok(())
}
