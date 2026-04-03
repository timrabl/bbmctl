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

pub mod campaign;
pub mod compare;
pub mod completions;
pub mod export;
pub mod history;
pub mod list;
pub mod provider;
pub mod report;
pub mod test;

use std::fs::File;
use std::io::{self, Write};

use anyhow::{Context, Result};

use crate::cli::ListArgs;

pub fn make_writer(args: &ListArgs) -> Result<Box<dyn Write>> {
    match &args.output {
        Some(path) => {
            let f = File::create(path)
                .with_context(|| format!("failed to create output file: {path}"))?;
            Ok(Box::new(f))
        }
        None => Ok(Box::new(io::stdout().lock())),
    }
}
