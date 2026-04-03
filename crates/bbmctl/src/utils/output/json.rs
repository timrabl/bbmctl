use anyhow::Result;
use serde::Serialize;
use std::io::Write;

pub fn write_json<T: Serialize>(writer: &mut dyn Write, data: &[T]) -> Result<()> {
    let formatted = serde_json::to_string_pretty(data)?;
    writeln!(writer, "{formatted}")?;
    Ok(())
}
