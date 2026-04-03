use std::io::Write;
use anyhow::Result;
use serde::Serialize;

pub fn write_csv<T: Serialize>(writer: &mut dyn Write, data: &[T], headers: bool) -> Result<()> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(headers)
        .from_writer(writer);

    for item in data {
        wtr.serialize(item)?;
    }

    wtr.flush()?;
    Ok(())
}
