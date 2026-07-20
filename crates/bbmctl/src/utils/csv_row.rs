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

//! Header-aware CSV rows for measurement import and export.
//!
//! Import previously read fields by position and ignored the header row
//! entirely, so the two schemas the tool itself emits -- `history export`
//! (starting at `timestamp`) and `history list -f csv` (starting at `id`) --
//! were mutually incompatible, and feeding one back in produced
//! `invalid float literal` with no row, column, or file named.

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;

/// A measurement as represented in CSV.
///
/// Every field is always present, unlike the database entity, whose
/// `skip_serializing_if` attributes make the serialized shape vary per row and
/// produce structurally malformed CSV.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeasurementCsvRow {
    pub timestamp: String,
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub latency_ms: f64,
    pub provider_id: String,
    pub plan_id: String,
}

impl MeasurementCsvRow {
    /// Build a fixed-width, spreadsheet-safe CSV row from a stored measurement.
    pub fn from_model(m: &bbmctl_database::entities::measurement::Model) -> Self {
        Self {
            timestamp: m.timestamp.clone(),
            download_kbps: m.download_kbps,
            upload_kbps: m.upload_kbps,
            latency_ms: m.latency_ms,
            provider_id: m.provider_id.map(|v| v.to_string()).unwrap_or_default(),
            plan_id: sanitize_csv_field(m.plan_id.as_deref().unwrap_or_default()),
        }
    }
}

/// A row parsed from an import file.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportRow {
    pub timestamp: String,
    pub download_kbps: f64,
    pub upload_kbps: f64,
    pub latency_ms: f64,
    pub provider_id: Option<i64>,
    pub plan_id: Option<String>,
}

/// Columns an import file must provide.
const REQUIRED: [&str; 4] = ["timestamp", "download_kbps", "upload_kbps", "latency_ms"];

/// Characters that make a spreadsheet treat a cell as a formula.
const FORMULA_PREFIXES: [char; 5] = ['=', '+', '-', '@', '\t'];

/// Neutralise a value that a spreadsheet would execute as a formula.
///
/// The csv crate quotes correctly, so this is not a parsing concern -- but
/// Excel and LibreOffice evaluate a leading `=`/`+`/`-`/`@` on open, which
/// turns an exported measurement history into an execution vector.
pub fn sanitize_csv_field(value: &str) -> String {
    match value.chars().next() {
        Some(c) if FORMULA_PREFIXES.contains(&c) => format!("'{value}"),
        _ => value.to_string(),
    }
}

/// Validate and normalise a timestamp for storage.
///
/// Import accepted anything, so a value SQLite could not parse reached the
/// database and permanently broke `history summary` and the metrics exporter.
pub fn parse_timestamp(value: &str) -> Result<String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|e| anyhow!("{e}"))
        .with_context(|| format!("invalid timestamp {value:?} (expected RFC 3339)"))?;

    Ok(parsed
        .with_timezone(&chrono::Utc)
        .format(bbmctl_database::TIMESTAMP_FORMAT)
        .to_string())
}

impl ImportRow {
    /// Parse one record, resolving fields by header name.
    ///
    /// `line` is the 1-based line number in the source file, used so an error
    /// points at the offending row rather than failing anonymously.
    pub fn from_record(
        headers: &csv::StringRecord,
        record: &csv::StringRecord,
        line: usize,
    ) -> Result<Self> {
        let index = |name: &str| -> Option<usize> { headers.iter().position(|h| h.trim() == name) };

        for name in REQUIRED {
            if index(name).is_none() {
                bail!(
                    "missing required column {name:?}; found columns: {}",
                    headers.iter().collect::<Vec<_>>().join(", ")
                );
            }
        }

        let field = |name: &str| -> &str {
            index(name)
                .and_then(|i| record.get(i))
                .unwrap_or_default()
                .trim()
        };

        let number = |name: &str| -> Result<f64> {
            let raw = field(name);
            raw.parse::<f64>()
                .with_context(|| format!("line {line}: column {name:?} has invalid number {raw:?}"))
        };

        let timestamp = parse_timestamp(field("timestamp"))
            .with_context(|| format!("line {line}: column \"timestamp\" is invalid"))?;

        let provider_id = match field("provider_id") {
            "" => None,
            raw => Some(raw.parse::<i64>().with_context(|| {
                format!("line {line}: column \"provider_id\" has invalid integer {raw:?}")
            })?),
        };

        let plan_id = match field("plan_id") {
            "" => None,
            raw => Some(raw.to_string()),
        };

        Ok(Self {
            timestamp,
            download_kbps: number("download_kbps")?,
            upload_kbps: number("upload_kbps")?,
            latency_ms: number("latency_ms")?,
            provider_id,
            plan_id,
        })
    }
}

impl From<&ImportRow> for bbmctl_database::NewMeasurement {
    fn from(r: &ImportRow) -> Self {
        Self {
            timestamp: r.timestamp.clone(),
            download_kbps: r.download_kbps,
            upload_kbps: r.upload_kbps,
            latency_ms: r.latency_ms,
            provider_id: r.provider_id,
            plan_id: r.plan_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(fields: &[&str]) -> csv::StringRecord {
        csv::StringRecord::from(fields.to_vec())
    }

    /// The schema `history export` writes must import cleanly.
    #[test]
    fn parses_the_export_schema() {
        let headers = record(&[
            "timestamp",
            "download_kbps",
            "upload_kbps",
            "latency_ms",
            "provider_id",
            "plan_id",
        ]);
        let row = record(&[
            "2026-07-20T09:56:55.000Z",
            "1000.5",
            "500.25",
            "5.5",
            "437",
            "plan-1",
        ]);

        let parsed = ImportRow::from_record(&headers, &row, 2).unwrap();

        assert_eq!(parsed.timestamp, "2026-07-20T09:56:55.000Z");
        assert_eq!(parsed.download_kbps, 1000.5);
        assert_eq!(parsed.provider_id, Some(437));
        assert_eq!(parsed.plan_id.as_deref(), Some("plan-1"));
    }

    /// The schema `history list -f csv` writes must import too -- it leads with
    /// `id` and has a different column order. Reading by position made these
    /// two self-produced formats mutually incompatible.
    #[test]
    fn parses_the_list_schema_with_leading_id() {
        let headers = record(&[
            "id",
            "timestamp",
            "download_kbps",
            "upload_kbps",
            "latency_ms",
            "provider_id",
            "plan_id",
        ]);
        let row = record(&[
            "7",
            "2026-07-20T09:56:55.000Z",
            "1000.5",
            "500.25",
            "5.5",
            "437",
            "plan-1",
        ]);

        let parsed = ImportRow::from_record(&headers, &row, 2).unwrap();

        assert_eq!(parsed.download_kbps, 1000.5, "columns resolved by name");
        assert_eq!(parsed.provider_id, Some(437));
    }

    /// Column order must not matter once names are used.
    #[test]
    fn column_order_is_irrelevant() {
        let headers = record(&[
            "plan_id",
            "latency_ms",
            "timestamp",
            "upload_kbps",
            "download_kbps",
        ]);
        let row = record(&["p1", "5.5", "2026-07-20T09:56:55.000Z", "500.25", "1000.5"]);

        let parsed = ImportRow::from_record(&headers, &row, 2).unwrap();

        assert_eq!(parsed.download_kbps, 1000.5);
        assert_eq!(parsed.latency_ms, 5.5);
        assert_eq!(parsed.plan_id.as_deref(), Some("p1"));
    }

    /// An unparseable timestamp must be rejected at the boundary rather than
    /// reaching the database, where it breaks `history summary` for good.
    #[test]
    fn rejects_invalid_timestamp_naming_the_row() {
        let headers = record(&["timestamp", "download_kbps", "upload_kbps", "latency_ms"]);
        let row = record(&["not-a-date", "1", "1", "1"]);

        let err = ImportRow::from_record(&headers, &row, 42).unwrap_err();
        let msg = format!("{err:#}");

        assert!(msg.contains("42"), "error should name the line: {msg}");
        assert!(
            msg.contains("timestamp"),
            "error should name the column: {msg}"
        );
    }

    /// A bad number must say which line and which column, not just
    /// "invalid float literal".
    #[test]
    fn rejects_invalid_number_naming_row_and_column() {
        let headers = record(&["timestamp", "download_kbps", "upload_kbps", "latency_ms"]);
        let row = record(&["2026-07-20T09:56:55.000Z", "abc", "1", "1"]);

        let err = ImportRow::from_record(&headers, &row, 13).unwrap_err();
        let msg = format!("{err:#}");

        assert!(msg.contains("13"), "error should name the line: {msg}");
        assert!(
            msg.contains("download_kbps"),
            "error should name the column: {msg}"
        );
    }

    #[test]
    fn missing_required_column_is_reported() {
        let headers = record(&["timestamp", "download_kbps"]);
        let row = record(&["2026-07-20T09:56:55.000Z", "1"]);

        let err = ImportRow::from_record(&headers, &row, 2).unwrap_err();
        let msg = format!("{err:#}");

        assert!(
            msg.contains("upload_kbps"),
            "should name what is missing: {msg}"
        );
    }

    #[test]
    fn empty_optional_fields_become_none() {
        let headers = record(&[
            "timestamp",
            "download_kbps",
            "upload_kbps",
            "latency_ms",
            "provider_id",
            "plan_id",
        ]);
        let row = record(&["2026-07-20T09:56:55.000Z", "1", "1", "1", "", ""]);

        let parsed = ImportRow::from_record(&headers, &row, 2).unwrap();

        assert_eq!(parsed.provider_id, None);
        assert_eq!(parsed.plan_id, None);
    }

    /// Timestamps in other valid RFC 3339 forms normalise to the canonical one.
    #[test]
    fn timestamps_are_normalised() {
        assert_eq!(
            parse_timestamp("2026-07-20T09:56:55.672667+00:00").unwrap(),
            "2026-07-20T09:56:55.672Z"
        );
        assert_eq!(
            parse_timestamp("2026-07-20T11:56:55+02:00").unwrap(),
            "2026-07-20T09:56:55.000Z"
        );
    }

    /// A leading formula character is executed on open by Excel and
    /// LibreOffice, so an exported history becomes an execution vector.
    #[test]
    fn formula_prefixes_are_neutralised() {
        assert_eq!(sanitize_csv_field("=cmd|calc"), "'=cmd|calc");
        assert_eq!(sanitize_csv_field("+1"), "'+1");
        assert_eq!(sanitize_csv_field("-1"), "'-1");
        assert_eq!(sanitize_csv_field("@SUM(A1)"), "'@SUM(A1)");
    }

    #[test]
    fn ordinary_values_are_untouched() {
        assert_eq!(sanitize_csv_field("plan-1"), "plan-1");
        assert_eq!(sanitize_csv_field("437"), "437");
        assert_eq!(sanitize_csv_field(""), "");
    }
}
