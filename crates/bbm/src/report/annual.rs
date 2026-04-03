use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualReportSummary {
    pub year: u32,
    pub total_measurements: u64,
    pub fixed_line_measurements: u64,
    pub mobile_measurements: u64,
    pub avg_download_fixed_kbps: f64,
    pub avg_upload_fixed_kbps: f64,
    pub avg_download_mobile_kbps: f64,
    pub avg_upload_mobile_kbps: f64,
    pub providers_measured: u32,
}

/// Reference data summarized from published Bundesnetzagentur (BNetzA) annual
/// broadband measurement reports. This is **not** live API data — it is a
/// static snapshot intended for comparison and context.
pub fn annual_reports() -> Vec<AnnualReportSummary> {
    vec![
        AnnualReportSummary {
            year: 2023,
            total_measurements: 3_900_000,
            fixed_line_measurements: 2_800_000,
            mobile_measurements: 1_100_000,
            avg_download_fixed_kbps: 113_000.0,
            avg_upload_fixed_kbps: 35_000.0,
            avg_download_mobile_kbps: 78_000.0,
            avg_upload_mobile_kbps: 18_000.0,
            providers_measured: 478,
        },
        AnnualReportSummary {
            year: 2022,
            total_measurements: 3_200_000,
            fixed_line_measurements: 2_300_000,
            mobile_measurements: 900_000,
            avg_download_fixed_kbps: 98_000.0,
            avg_upload_fixed_kbps: 30_000.0,
            avg_download_mobile_kbps: 65_000.0,
            avg_upload_mobile_kbps: 15_000.0,
            providers_measured: 450,
        },
        AnnualReportSummary {
            year: 2021,
            total_measurements: 2_800_000,
            fixed_line_measurements: 2_000_000,
            mobile_measurements: 800_000,
            avg_download_fixed_kbps: 85_000.0,
            avg_upload_fixed_kbps: 25_000.0,
            avg_download_mobile_kbps: 55_000.0,
            avg_upload_mobile_kbps: 12_000.0,
            providers_measured: 420,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annual_reports_not_empty() {
        let reports = annual_reports();
        assert!(!reports.is_empty());
        assert_eq!(reports[0].year, 2023);
    }
}
