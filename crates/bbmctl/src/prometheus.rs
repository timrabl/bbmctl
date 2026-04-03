use std::fmt::Write as _;

use anyhow::{Context, Result};
use log::info;
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

pub async fn serve(port: u16, db: &Database) -> Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    info!("prometheus exporter listening on http://{addr}/metrics");
    info!("press Ctrl+C to stop");

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                let (stream, _peer) = accept_result?;
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);

                // Read the request line to determine the path
                let mut request_line = String::new();
                buf_reader.read_line(&mut request_line).await?;

                // Consume remaining headers
                loop {
                    let mut line = String::new();
                    buf_reader.read_line(&mut line).await?;
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                }

                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/");

                if path == "/metrics" || path == "/" {
                    match render_metrics(db).await {
                        Ok(body) => {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = writer.write_all(response.as_bytes()).await;
                        }
                        Err(e) => {
                            let body = format!("error: {e}");
                            let response = format!(
                                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = writer.write_all(response.as_bytes()).await;
                        }
                    }
                } else {
                    let body = "not found";
                    let response = format!(
                        "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = writer.write_all(response.as_bytes()).await;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("prometheus exporter stopped");
                break;
            }
        }
    }

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
