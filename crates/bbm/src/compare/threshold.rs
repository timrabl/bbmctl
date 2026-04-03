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
                met: true,
                percent: 0.0,
            },
        }
    }
}
