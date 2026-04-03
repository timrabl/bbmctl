use anyhow::Result;
use serde::Serialize;
use std::io::Write;

pub fn write_yaml<T: Serialize>(writer: &mut dyn Write, data: &[T]) -> Result<()> {
    let formatted = serde_yaml_ng::to_string(data)?;
    write!(writer, "{formatted}")?;
    Ok(())
}
