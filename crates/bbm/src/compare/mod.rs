mod comparison;
mod row;
mod threshold;

pub use comparison::ComparisonResult;
pub use row::ComparisonRow;
pub use threshold::ThresholdResult;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intstr::InconsistentIntegerString as IIS;
    use crate::plan::Plan;

    fn make_plan(
        min_dl: &str,
        norm_dl: &str,
        max_dl: &str,
        min_ul: &str,
        norm_ul: &str,
        max_ul: &str,
    ) -> Plan {
        Plan {
            plan_id: IIS("1".into()),
            plan_db_version: IIS("1".into()),
            mindownload: IIS(min_dl.into()),
            normdownload: IIS(norm_dl.into()),
            maxdownload: IIS(max_dl.into()),
            maxdownloadpub: IIS(max_dl.into()),
            minupload: IIS(min_ul.into()),
            normupload: IIS(norm_ul.into()),
            maxupload: IIS(max_ul.into()),
            product_highlighted: IIS("0".into()),
        }
    }

    #[test]
    fn all_thresholds_met() {
        let plan = make_plan("50000", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(100_000.0, 40_000.0);
        assert!(result.all_met);
        assert!(result.results.iter().all(|r| r.met));
    }

    #[test]
    fn below_minimum() {
        let plan = make_plan("50000", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(30_000.0, 5_000.0);
        assert!(!result.all_met);

        let min_dl = &result.results[0];
        assert!(!min_dl.met);
        assert_eq!(min_dl.percent, 60.0);
    }

    #[test]
    fn between_normal_and_max() {
        let plan = make_plan("50000", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(90_000.0, 30_000.0);
        assert!(!result.all_met);

        assert!(result.results[0].met); // >= min
        assert!(result.results[1].met); // >= normal
        assert!(!result.results[2].met); // >= max
    }

    #[test]
    fn flatten_skips_no_threshold() {
        let plan = make_plan("50000", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(100_000.0, 40_000.0);
        let rows = ComparisonResult::flatten(&[result]);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().all(|r| r.met == "PASS"));
    }
}
