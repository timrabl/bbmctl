mod csv;
mod json;
mod table;
mod trend;
mod yaml;

use crate::cli::OutputFormat;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

pub use trend::render_trend;

pub fn write_output<T: Serialize>(
    writer: &mut dyn Write,
    data: &[T],
    format: &OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Table => table::write_table(writer, data),
        OutputFormat::Json => json::write_json(writer, data),
        OutputFormat::Yaml => yaml::write_yaml(writer, data),
        OutputFormat::Csv => csv::write_csv(writer, data, true),
        OutputFormat::CsvHeadless => csv::write_csv(writer, data, false),
    }
}
