use crate::utils::speed_fmt::{format_speed, SpeedUnit};

const SPARKLINE_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sparkline from a slice of f64 values.
fn sparkline(values: &[f64]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    values
        .iter()
        .map(|&v| {
            if range == 0.0 {
                SPARKLINE_CHARS[3] // middle bar when all values equal
            } else {
                let normalized = (v - min) / range;
                let idx = (normalized * (SPARKLINE_CHARS.len() - 1) as f64).round() as usize;
                SPARKLINE_CHARS[idx.min(SPARKLINE_CHARS.len() - 1)]
            }
        })
        .collect()
}

/// Render trend output for download, upload, and latency.
pub fn render_trend(
    download_kbps: &[f64],
    upload_kbps: &[f64],
    latency_ms: &[f64],
    unit: &SpeedUnit,
) -> String {
    let mut out = String::new();

    if download_kbps.is_empty() {
        return out;
    }

    let n = download_kbps.len();

    // Download
    let dl_spark = sparkline(download_kbps);
    let dl_avg = download_kbps.iter().sum::<f64>() / n as f64;
    let dl_min = download_kbps.iter().cloned().fold(f64::INFINITY, f64::min);
    let dl_max = download_kbps
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    out.push_str(&format!(
        "download: {dl_spark}  avg: {}  min: {}  max: {}\n",
        format_speed(dl_avg, unit),
        format_speed(dl_min, unit),
        format_speed(dl_max, unit),
    ));

    // Upload
    let ul_spark = sparkline(upload_kbps);
    let ul_avg = upload_kbps.iter().sum::<f64>() / n as f64;
    let ul_min = upload_kbps.iter().cloned().fold(f64::INFINITY, f64::min);
    let ul_max = upload_kbps
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    out.push_str(&format!(
        "upload:   {ul_spark}  avg: {}  min: {}  max: {}\n",
        format_speed(ul_avg, unit),
        format_speed(ul_min, unit),
        format_speed(ul_max, unit),
    ));

    // Latency (no speed formatting, just ms)
    let lat_spark = sparkline(latency_ms);
    let lat_avg = latency_ms.iter().sum::<f64>() / n as f64;
    let lat_min = latency_ms.iter().cloned().fold(f64::INFINITY, f64::min);
    let lat_max = latency_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    out.push_str(&format!(
        "latency:  {lat_spark}  avg: {lat_avg:.1} ms  min: {lat_min:.1} ms  max: {lat_max:.1} ms\n",
    ));

    out.push_str(&format!("({n} measurements)\n"));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_empty() {
        assert_eq!(sparkline(&[]), "");
    }

    #[test]
    fn sparkline_single() {
        assert_eq!(sparkline(&[50.0]), "▄");
    }

    #[test]
    fn sparkline_two_values() {
        let s = sparkline(&[0.0, 100.0]);
        assert_eq!(s, "▁█");
    }

    #[test]
    fn sparkline_equal_values() {
        let s = sparkline(&[50.0, 50.0, 50.0]);
        assert_eq!(s, "▄▄▄");
    }

    #[test]
    fn render_trend_basic() {
        let dl = vec![50000.0, 48000.0, 52000.0];
        let ul = vec![25000.0, 24000.0, 26000.0];
        let lat = vec![15.0, 14.0, 16.0];

        let output = render_trend(&dl, &ul, &lat, &SpeedUnit::Auto);
        assert!(output.contains("download:"));
        assert!(output.contains("upload:"));
        assert!(output.contains("latency:"));
        assert!(output.contains("(3 measurements)"));
    }
}
