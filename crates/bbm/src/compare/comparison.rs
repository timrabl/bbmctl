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

use serde::Serialize;

use super::{ComparisonRow, ThresholdResult};

#[derive(Debug, Clone, Serialize)]
pub struct ComparisonResult {
    pub plan_id: String,
    pub results: Vec<ThresholdResult>,
    pub all_met: bool,
}

impl ComparisonResult {
    /// Flatten multiple comparison results into rows for table/csv output,
    /// skipping checks where no threshold was defined.
    pub fn flatten(results: &[Self]) -> Vec<ComparisonRow> {
        let mut rows = Vec::new();
        for cr in results {
            for tr in &cr.results {
                if tr.threshold_kbps.is_none() {
                    continue;
                }
                rows.push(ComparisonRow {
                    plan_id: cr.plan_id.clone(),
                    check: tr.check.clone(),
                    threshold_kbps: tr.threshold_kbps.map(|v| v.to_string()).unwrap_or_default(),
                    threshold_display: tr.threshold_display.clone().unwrap_or_default(),
                    measured_kbps: tr.measured_kbps,
                    measured_display: tr.measured_display.clone(),
                    percent: format!("{}%", tr.percent),
                    met: if tr.met { "PASS" } else { "FAIL" }.into(),
                });
            }
        }
        rows
    }
}
