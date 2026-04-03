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

#[derive(Debug, Clone)]
pub struct SpeedTestConfig {
    pub peer: String,
    pub rtt_peer: String,
    pub duration_secs: u64,
    pub port: u16,
    pub streams: u16,
}

impl SpeedTestConfig {
    pub const DEFAULT_PEER: &str = "drsfv4.breitbandmessung.de";
    pub const DEFAULT_RTT_PEER: &str = "drsfrtt.breitbandmessung.de";
    pub const DEFAULT_DURATION_SECS: u64 = 10;
    pub const DEFAULT_PORT: u16 = 443;
    pub const DEFAULT_STREAMS: u16 = 8;
}

impl Default for SpeedTestConfig {
    fn default() -> Self {
        Self {
            peer: Self::DEFAULT_PEER.to_string(),
            rtt_peer: Self::DEFAULT_RTT_PEER.to_string(),
            duration_secs: Self::DEFAULT_DURATION_SECS,
            port: Self::DEFAULT_PORT,
            streams: Self::DEFAULT_STREAMS,
        }
    }
}
