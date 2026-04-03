use serde::{Deserialize, Serialize};

use crate::intstr::InconsistentIntegerString;
use crate::compare::{ComparisonResult, ThresholdResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: InconsistentIntegerString,
    pub plan_db_version: InconsistentIntegerString,
    pub mindownload: InconsistentIntegerString,
    pub normdownload: InconsistentIntegerString,
    pub maxdownload: InconsistentIntegerString,
    pub maxdownloadpub: InconsistentIntegerString,
    pub minupload: InconsistentIntegerString,
    pub normupload: InconsistentIntegerString,
    pub maxupload: InconsistentIntegerString,
    pub product_highlighted: InconsistentIntegerString,
}

impl Plan {
    fn parse_speed(field: &InconsistentIntegerString) -> Option<f64> {
        field.0.parse::<f64>().ok()
    }

    /// Minimum contractual download speed in kbit/s.
    pub fn min_download_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.mindownload)
    }

    /// Normal (typically available) download speed in kbit/s.
    pub fn norm_download_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.normdownload)
    }

    /// Maximum contractual download speed in kbit/s.
    pub fn max_download_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.maxdownload)
    }

    /// Minimum contractual upload speed in kbit/s.
    pub fn min_upload_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.minupload)
    }

    /// Normal (typically available) upload speed in kbit/s.
    pub fn norm_upload_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.normupload)
    }

    /// Maximum contractual upload speed in kbit/s.
    pub fn max_upload_kbps(&self) -> Option<f64> {
        Self::parse_speed(&self.maxupload)
    }

    /// Compare measured speeds against this plan's contractual thresholds.
    pub fn compare(&self, download_kbps: f64, upload_kbps: f64) -> ComparisonResult {
        let results = vec![
            ThresholdResult::check("download >= min", download_kbps, self.min_download_kbps()),
            ThresholdResult::check("download >= normal", download_kbps, self.norm_download_kbps()),
            ThresholdResult::check("download >= max", download_kbps, self.max_download_kbps()),
            ThresholdResult::check("upload >= min", upload_kbps, self.min_upload_kbps()),
            ThresholdResult::check("upload >= normal", upload_kbps, self.norm_upload_kbps()),
            ThresholdResult::check("upload >= max", upload_kbps, self.max_upload_kbps()),
        ];

        let all_met = results.iter().all(|r| r.met);

        ComparisonResult {
            plan_id: self.plan_id.0.clone(),
            results,
            all_met,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::BbmClient;

    const TESTING_PROVIDER_IDS: &[i64] = &[1, 7, 10, 11, 244, 251, 330, 416, 709];

    #[tokio::test]
    #[ignore] // hits live API
    async fn test_get_plans_by_provider_id() {
        let client = BbmClient::new();
        for &id in TESTING_PROVIDER_IDS {
            let plans = client.get_plans_by_provider_id(id).await.unwrap();
            for plan in &plans {
                assert!(!plan.plan_id.0.is_empty());
            }
        }
    }
}
