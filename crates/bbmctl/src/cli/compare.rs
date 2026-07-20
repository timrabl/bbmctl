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

use clap::Args;

use super::ListArgs;

/// Compare measured speeds against a contractual plan
///
/// Exit codes: 0 = all thresholds met, 1 = error, 2 = one or more thresholds not met
#[derive(Args, Clone)]
pub struct CompareArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Provider ID. Falls back to the config file or `provider switch`.
    #[arg(long)]
    pub provider: Option<i64>,

    /// Plan ID (if omitted, compares against all plans for the provider)
    #[arg(long)]
    pub plan: Option<String>,

    /// Measured download speed in kbit/s (omit to run a speed test)
    #[arg(long, requires = "upload")]
    pub download: Option<f64>,

    /// Measured upload speed in kbit/s (omit to run a speed test)
    #[arg(long, requires = "download")]
    pub upload: Option<f64>,

    /// Run a speed test before comparing (implied when --download/--upload omitted)
    #[arg(long)]
    pub test: bool,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,
}
