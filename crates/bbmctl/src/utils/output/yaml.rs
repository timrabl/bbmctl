use std::io::Write;
use anyhow::Result;
use serde::Serialize;

pub fn write_yaml<T: Serialize>(writer: &mut dyn Write, data: &[T]) -> Result<()> {
    let formatted = serde_yml::to_string(data)?;
    write!(writer, "{formatted}")?;
    Ok(())
}
