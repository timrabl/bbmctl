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

use anyhow::Result;
use log::{info, warn};

use super::make_writer;
use crate::cli::ReportCommands;
use crate::utils::output;

pub async fn run(command: ReportCommands) -> Result<()> {
    match command {
        ReportCommands::Annual(args) => {
            let mut reports = bbm::BbmClient::annual_reports();
            if let Some(year) = args.year {
                reports.retain(|r| r.year == year);
            }
            if reports.is_empty() {
                info!("no report data found for the specified filter");
            } else {
                let mut writer = make_writer(&args.list)?;
                output::write_output(&mut writer, &reports, &args.list.format)?;
            }
        }
        ReportCommands::Stats(args) => {
            let client = bbm::BbmClient::new();
            match client.get_statistics().await {
                Ok(stats) => {
                    let mut writer = make_writer(&args.list)?;
                    output::write_output(&mut writer, &[stats], &args.list.format)?;
                }
                Err(e) => {
                    warn!("could not fetch live statistics: {e}");
                    info!("this endpoint may not be publicly available");
                    info!("use `report annual` for published report data instead");
                }
            }
        }
    }
    Ok(())
}
