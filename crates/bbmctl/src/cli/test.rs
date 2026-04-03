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

#[derive(Args, Clone)]
pub struct TestArgs {
    #[command(flatten)]
    pub list: ListArgs,

    /// Test duration per phase in seconds
    #[arg(long, default_value = "10")]
    pub duration: u64,

    /// Number of concurrent streams for throughput measurement
    #[arg(long, default_value = "8")]
    pub streams: u16,

    /// Speed display unit
    #[arg(long, default_value = "auto")]
    pub unit: crate::utils::speed_fmt::SpeedUnit,

    /// Measurement peer hostname (IPv4)
    #[arg(long)]
    pub peer: Option<String>,

    /// Run repeatedly at this interval (e.g. 30m, 1h, 2h30m). Implies --record.
    #[arg(long)]
    pub every: Option<String>,

    /// Also record the result to the local database
    #[arg(long)]
    pub record: bool,

    /// Provider ID to associate with the recorded measurement
    #[arg(long, requires = "record")]
    pub provider: Option<i64>,

    /// Plan ID to associate with the recorded measurement
    #[arg(long, requires = "record")]
    pub plan: Option<String>,
}
