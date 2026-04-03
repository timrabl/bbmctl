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
use bbmctl_database::Database;
use log::info;

use crate::config::ResolvedConfig;

pub async fn run(
    command: crate::cli::ProviderCommands,
    config: &ResolvedConfig,
    db: &Database,
) -> Result<()> {
    match command {
        crate::cli::ProviderCommands::Switch(args) => {
            db.settings()
                .set("active_provider", &args.id.to_string())
                .await?;
            info!("active provider set to {}", args.id);
        }
        crate::cli::ProviderCommands::Show => {
            let active = db.settings().get("active_provider").await?;
            match active {
                Some(id) => println!("active provider: {id} (from database)"),
                None => match config.provider {
                    Some(id) => println!("active provider: {id} (from config)"),
                    None => println!("no active provider set"),
                },
            }
        }
    }
    Ok(())
}
