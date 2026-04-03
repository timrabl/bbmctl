use std::collections::BTreeSet;
use std::io::Write;
use anyhow::{Context, Result};
use comfy_table::{Row, Table};
use serde::Serialize;

pub fn write_table<T: Serialize>(writer: &mut dyn Write, data: &[T]) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    let objects: Vec<serde_json::Value> = data
        .iter()
        .map(serde_json::to_value)
        .collect::<serde_json::Result<_>>()
        .context("failed to serialize data for table output")?;

    let all_keys: BTreeSet<&str> = objects
        .iter()
        .filter_map(|o| o.as_object())
        .flat_map(|m| m.keys().map(String::as_str))
        .collect();

    let headers: Vec<&str> = all_keys.into_iter().collect();

    let mut table = Table::new();
    table.set_header(Row::from(headers.clone()));

    for obj in &objects {
        if let Some(map) = obj.as_object() {
            let row: Vec<String> = headers
                .iter()
                .map(|h| match map.get(*h) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Null) | None => String::new(),
                    Some(other) => other.to_string(),
                })
                .collect();
            table.add_row(Row::from(row));
        }
    }

    writeln!(writer, "{table}")?;
    Ok(())
}
