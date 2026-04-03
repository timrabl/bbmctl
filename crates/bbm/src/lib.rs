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

pub mod client;
pub mod compare;
pub mod error;
pub mod intstr;
pub mod plan;
pub mod provider;
pub mod report;
pub mod retry;
pub mod speed;
pub mod speedtest;

pub use client::BbmClient;
pub use compare::{ComparisonResult, ComparisonRow, ThresholdResult};
pub use error::{BbmError, Result};
pub use intstr::InconsistentIntegerString;
pub use plan::Plan;
pub use provider::Provider;
pub use report::AnnualReportSummary;
pub use retry::RetryPolicy;
pub use speed::Speed;
pub use speedtest::{SpeedTestConfig, SpeedTestResult, SpeedTestRunner};
