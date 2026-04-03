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
