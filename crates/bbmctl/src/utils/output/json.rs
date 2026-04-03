use std::io::Write;
use anyhow::Result;
use serde::Serialize;

pub fn write_json<T: Serialize>(writer: &mut dyn Write, data: &[T]) -> Result<()> {
    let formatted = serde_json::to_string_pretty(data)?;
    writeln!(writer, "{formatted}")?;
    Ok(())
}
