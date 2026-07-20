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

use std::fmt::Write as _;
use std::time::Duration;

use anyhow::{Context, Result};
use log::{debug, info, warn};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use bbmctl_database::Database;

async fn render_metrics(db: &Database) -> Result<String> {
    let mut out = String::new();

    if let Some(m) = db.measurements().history(Some(1)).await?.into_iter().next() {
        writeln!(
            out,
            "# HELP speedtest_download_kbps Latest download speed in kbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_download_kbps gauge")?;
        writeln!(out, "speedtest_download_kbps {}", m.download_kbps)?;

        writeln!(
            out,
            "# HELP speedtest_upload_kbps Latest upload speed in kbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_upload_kbps gauge")?;
        writeln!(out, "speedtest_upload_kbps {}", m.upload_kbps)?;

        writeln!(
            out,
            "# HELP speedtest_latency_ms Latest latency in milliseconds"
        )?;
        writeln!(out, "# TYPE speedtest_latency_ms gauge")?;
        writeln!(out, "speedtest_latency_ms {}", m.latency_ms)?;

        writeln!(
            out,
            "# HELP speedtest_download_mbps Latest download speed in Mbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_download_mbps gauge")?;
        writeln!(
            out,
            "speedtest_download_mbps {:.2}",
            m.download_kbps / 1000.0
        )?;

        writeln!(
            out,
            "# HELP speedtest_upload_mbps Latest upload speed in Mbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_upload_mbps gauge")?;
        writeln!(out, "speedtest_upload_mbps {:.2}", m.upload_kbps / 1000.0)?;
    }

    if let Some(s) = db.measurements().summary().await? {
        writeln!(
            out,
            "# HELP speedtest_measurements_total Total number of recorded measurements"
        )?;
        writeln!(out, "# TYPE speedtest_measurements_total counter")?;
        writeln!(out, "speedtest_measurements_total {}", s.count)?;

        writeln!(
            out,
            "# HELP speedtest_avg_download_kbps Average download speed in kbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_avg_download_kbps gauge")?;
        writeln!(out, "speedtest_avg_download_kbps {}", s.avg_download_kbps)?;

        writeln!(
            out,
            "# HELP speedtest_avg_upload_kbps Average upload speed in kbit/s"
        )?;
        writeln!(out, "# TYPE speedtest_avg_upload_kbps gauge")?;
        writeln!(out, "speedtest_avg_upload_kbps {}", s.avg_upload_kbps)?;

        writeln!(
            out,
            "# HELP speedtest_avg_latency_ms Average latency in milliseconds"
        )?;
        writeln!(out, "# TYPE speedtest_avg_latency_ms gauge")?;
        writeln!(out, "speedtest_avg_latency_ms {}", s.avg_latency_ms)?;
    }

    Ok(out)
}

/// How long a client has to send a complete request head before its
/// connection is dropped. Without this, one half-open connection blocked every
/// other scrape.
const HEADER_READ_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn serve(bind: &str, port: u16, db: &Database) -> Result<()> {
    let addr = format!("{bind}:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    info!("prometheus exporter listening on http://{addr}/metrics");
    if bind == "0.0.0.0" || bind == "::" {
        warn!("exporter is reachable from the network and has no authentication");
    }
    info!("press Ctrl+C to stop");

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                // A failed accept must not end the process.
                let Ok((stream, peer)) = accept_result else {
                    warn!("failed to accept connection");
                    continue;
                };

                // Serve each client on its own task. Handling connections
                // inline meant a single slow or stuck client starved all
                // other scrapes.
                let metrics = render_metrics(db).await;
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, metrics).await {
                        debug!("connection from {peer} ended: {e}");
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => {
                info!("prometheus exporter stopped");
                break;
            }
        }
    }

    Ok(())
}

/// Serve one client. Errors are confined to this connection: a client that
/// resets mid-request used to propagate out of `serve` and kill the exporter.
async fn handle_connection(
    stream: tokio::net::TcpStream,
    metrics: Result<String>,
) -> std::io::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let mut request_line = String::new();
    let head = tokio::time::timeout(HEADER_READ_TIMEOUT, async {
        buf_reader.read_line(&mut request_line).await?;
        loop {
            let mut line = String::new();
            let n = buf_reader.read_line(&mut line).await?;
            if n == 0 || line == "\r\n" || line.is_empty() {
                break;
            }
        }
        Ok::<_, std::io::Error>(())
    })
    .await;

    match head {
        Ok(Ok(())) => {}
        // Timed out or failed mid-headers: drop this client only.
        _ => return Ok(()),
    }

    let path = request_line.split_whitespace().nth(1).unwrap_or("/");

    let response = if path == "/metrics" || path == "/" {
        match metrics {
            Ok(body) => format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            ),
            Err(e) => {
                let body = format!("error: {e}");
                format!(
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
            }
        }
    } else {
        let body = "not found";
        format!(
            "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    };

    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn render_empty_db() {
        let db = Database::connect_in_memory().await.unwrap();
        let metrics = render_metrics(&db).await.unwrap();
        assert!(metrics.is_empty());
    }

    #[tokio::test]
    async fn render_with_data() {
        let db = Database::connect_in_memory().await.unwrap();
        db.measurements()
            .record(100_000.0, 50_000.0, 12.5, None, None)
            .await
            .unwrap();

        let metrics = render_metrics(&db).await.unwrap();
        assert!(metrics.contains("speedtest_download_kbps 100000"));
        assert!(metrics.contains("speedtest_upload_kbps 50000"));
        assert!(metrics.contains("speedtest_latency_ms 12.5"));
        assert!(metrics.contains("speedtest_download_mbps 100.00"));
        assert!(metrics.contains("speedtest_measurements_total 1"));
    }
}

#[cfg(test)]
mod serve_tests {
    use super::*;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpStream;

    async fn scrape(addr: &str) -> Option<String> {
        let mut sock = TcpStream::connect(addr).await.ok()?;
        sock.write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\n\r\n")
            .await
            .ok()?;
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut sock, &mut buf)
            .await
            .ok()?;
        Some(String::from_utf8_lossy(&buf).to_string())
    }

    /// Connections were handled inline in the accept loop with no read
    /// timeout, so a single client that opened a socket and never finished its
    /// request headers blocked every other scrape indefinitely.
    #[tokio::test]
    async fn half_open_connection_does_not_block_scrapes() {
        let db = Database::connect_in_memory().await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener);

        let port: u16 = addr.rsplit(':').next().unwrap().parse().unwrap();
        tokio::spawn(async move {
            let db = db;
            let _ = serve("127.0.0.1", port, &db).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Occupy the server with a request that never terminates its headers.
        let mut stuck = TcpStream::connect(&addr).await.unwrap();
        stuck
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\n")
            .await
            .unwrap();

        // A well-behaved client must still be served promptly.
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), scrape(&addr)).await;

        assert!(
            result.is_ok(),
            "a half-open connection blocked all other scrapes"
        );
        assert!(
            result.unwrap().unwrap_or_default().contains("HTTP/1.1 200"),
            "scrape did not return a successful response"
        );
    }

    /// A client that resets mid-headers must not terminate the exporter.
    #[tokio::test]
    async fn abrupt_disconnect_does_not_kill_the_exporter() {
        let db = Database::connect_in_memory().await.unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let addr = format!("127.0.0.1:{port}");

        tokio::spawn(async move {
            let db = db;
            let _ = serve("127.0.0.1", port, &db).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // Connect and immediately drop, mid-request.
        {
            let mut sock = TcpStream::connect(&addr).await.unwrap();
            sock.write_all(b"GET /met").await.unwrap();
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let after = tokio::time::timeout(std::time::Duration::from_secs(5), scrape(&addr)).await;
        assert!(
            after.is_ok() && after.unwrap().is_some(),
            "exporter died after a client disconnected mid-request"
        );
    }
}
