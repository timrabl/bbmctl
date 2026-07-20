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

    /// An unparseable field (the API has returned "", "n/a", and comma
    /// decimals) must never let a run be reported as passing. This is used as
    /// evidence in a BNetzA Nachweisverfahren, so a false "ALL PASS" is the
    /// worst possible failure mode.
    #[test]
    fn unparseable_threshold_does_not_pass() {
        let plan = make_plan("", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(100_000.0, 40_000.0);

        assert!(
            !result.all_met,
            "a plan with an unverifiable term must not report all_met"
        );
    }

    /// ...but every term that could be checked passing must still be
    /// distinguishable from an outright failure.
    #[test]
    fn unparseable_threshold_is_skipped_not_failed() {
        let plan = make_plan("", "80000", "100000", "10000", "20000", "40000");
        let result = plan.compare(100_000.0, 40_000.0);

        let checkable: Vec<_> = result
            .results
            .iter()
            .filter(|r| r.threshold_kbps.is_some())
            .collect();
        assert!(
            checkable.iter().all(|r| r.met),
            "every checkable term should have passed"
        );
        assert_eq!(checkable.len(), 5, "one of six terms was unparseable");
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
