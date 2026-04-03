// Copyright (c) 2023-2026 Tim Oliver Rabl
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use anyhow::{Context, Result};
use comfy_table::{Row, Table};
use serde::Serialize;
use std::collections::BTreeSet;
use std::io::Write;

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
