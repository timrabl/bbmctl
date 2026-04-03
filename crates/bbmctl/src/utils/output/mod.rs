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
