pub mod campaign;
pub mod compare;
pub mod completions;
pub mod export;
pub mod history;
pub mod list;
pub mod provider;
pub mod report;
pub mod test;

use std::fs::File;
use std::io::{self, Write};

use anyhow::{Context, Result};

use crate::cli::ListArgs;

pub fn make_writer(args: &ListArgs) -> Result<Box<dyn Write>> {
    match &args.output {
        Some(path) => {
            let f = File::create(path)
                .with_context(|| format!("failed to create output file: {path}"))?;
            Ok(Box::new(f))
        }
        None => Ok(Box::new(io::stdout().lock())),
    }
}
