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

use clap::ValueEnum;
use serde::Serialize;

#[derive(Debug, Clone, Default, ValueEnum)]
pub enum SpeedUnit {
    #[default]
    Auto,
    Kbps,
    Mbps,
    Gbps,
}

pub fn format_speed(kbps: f64, unit: &SpeedUnit) -> String {
    match unit {
        SpeedUnit::Kbps => format!("{kbps:.1} kbit/s"),
        SpeedUnit::Mbps => format!("{:.2} Mbit/s", kbps / 1000.0),
        SpeedUnit::Gbps => format!("{:.3} Gbit/s", kbps / 1_000_000.0),
        SpeedUnit::Auto => {
            if kbps >= 1_000_000.0 {
                format!("{:.3} Gbit/s", kbps / 1_000_000.0)
            } else if kbps >= 1000.0 {
                format!("{:.2} Mbit/s", kbps / 1000.0)
            } else {
                format!("{kbps:.1} kbit/s")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FormattedSpeedTestResult {
    pub download: String,
    pub upload: String,
    pub latency_ms: f64,
    pub jitter_ms: f64,
    pub peer: String,
    pub duration_secs: u64,
    pub streams: u16,
}

impl FormattedSpeedTestResult {
    pub fn from_result(result: &bbm::SpeedTestResult, unit: &SpeedUnit) -> Self {
        Self {
            download: format_speed(result.download_kbps, unit),
            upload: format_speed(result.upload_kbps, unit),
            latency_ms: result.latency_ms,
            jitter_ms: result.jitter_ms,
            peer: result.peer.clone(),
            duration_secs: result.duration_secs,
            streams: result.streams,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_selects_kbps() {
        assert_eq!(format_speed(500.0, &SpeedUnit::Auto), "500.0 kbit/s");
    }

    #[test]
    fn auto_selects_mbps() {
        assert_eq!(format_speed(50000.0, &SpeedUnit::Auto), "50.00 Mbit/s");
    }

    #[test]
    fn auto_selects_gbps() {
        assert_eq!(format_speed(1_500_000.0, &SpeedUnit::Auto), "1.500 Gbit/s");
    }

    #[test]
    fn forced_kbps() {
        assert_eq!(format_speed(50000.0, &SpeedUnit::Kbps), "50000.0 kbit/s");
    }

    #[test]
    fn forced_mbps() {
        assert_eq!(format_speed(500.0, &SpeedUnit::Mbps), "0.50 Mbit/s");
    }

    #[test]
    fn forced_gbps() {
        assert_eq!(format_speed(50000.0, &SpeedUnit::Gbps), "0.050 Gbit/s");
    }
}
