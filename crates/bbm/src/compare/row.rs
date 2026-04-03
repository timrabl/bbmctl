use serde::Serialize;

/// Flat row for table/csv output — one row per threshold check.
#[derive(Debug, Clone, Serialize)]
pub struct ComparisonRow {
    pub plan_id: String,
    pub check: String,
    pub threshold_kbps: String,
    pub threshold_display: String,
    pub measured_kbps: f64,
    pub measured_display: String,
    pub percent: String,
    pub met: String,
}
