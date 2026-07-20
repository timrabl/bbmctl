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

/// Format kbit/s as human-readable Mbit/s.
fn fmt_mbps(kbps: f64) -> String {
    format!("{:.1} Mbit/s", kbps / 1000.0)
}

#[derive(Debug, Clone, Serialize)]
pub struct ThresholdResult {
    pub check: String,
    pub threshold_kbps: Option<f64>,
    pub threshold_display: Option<String>,
    pub measured_kbps: f64,
    pub measured_display: String,
    pub met: bool,
    pub percent: f64,
}

impl ThresholdResult {
    /// Compare a measured value against an optional threshold.
    pub fn check(label: &str, measured: f64, threshold: Option<f64>) -> Self {
        match threshold {
            Some(t) => {
                let met = measured >= t;
                let pct = if t > 0.0 {
                    (measured / t * 100.0).round()
                } else {
                    100.0
                };
                Self {
                    check: label.into(),
                    threshold_kbps: Some(t),
                    threshold_display: Some(fmt_mbps(t)),
                    measured_kbps: measured,
                    measured_display: fmt_mbps(measured),
                    met,
                    percent: pct,
                }
            }
            None => Self {
                check: label.into(),
                threshold_kbps: None,
                threshold_display: None,
                measured_kbps: measured,
                measured_display: fmt_mbps(measured),
                // No threshold means the term could not be checked. Reporting
                // `true` would claim a contract term was satisfied when it was
                // never verified.
                met: false,
                percent: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A missing threshold means "could not be checked", not "passed". Folding
    /// it into `met: true` tells the user their line satisfies a contract term
    /// that was never actually verified -- and `Plan::parse_speed` discards
    /// parse errors, so an API returning "" or "50,000" lands here.
    #[test]
    fn absent_threshold_is_not_reported_as_met() {
        let result = ThresholdResult::check("download >= min", 50_000.0, None);

        assert!(
            !result.met,
            "an unverifiable threshold must not be reported as met"
        );
    }

    #[test]
    fn present_threshold_still_compares_normally() {
        let pass = ThresholdResult::check("download >= min", 50_000.0, Some(40_000.0));
        assert!(pass.met);

        let fail = ThresholdResult::check("download >= min", 30_000.0, Some(40_000.0));
        assert!(!fail.met);
    }
}
