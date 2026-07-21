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

//! Minimal in-process HTTP stub server for tests.
//!
//! Deliberately built on `tokio::net` rather than a mocking crate: the HTTP
//! layer is what we need to test, so the fewer moving parts between the test
//! and the socket, the better. It also keeps `bbm` free of dev-dependencies.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A stub HTTP server bound to an ephemeral localhost port.
pub struct StubServer {
    /// Base URL to hand to `BbmClient::with_base_url`.
    pub base_url: String,
    hits: Arc<AtomicUsize>,
}

impl StubServer {
    /// Serve a fixed raw HTTP response to every connection.
    pub async fn serve_raw(response: impl Into<Vec<u8>>) -> Self {
        let response = response.into();
        Self::spawn(move |_n| Some(response.clone())).await
    }

    /// Serve a response built from the request index (0-based), so a test can
    /// vary behaviour per attempt. Returning `None` closes the connection
    /// without writing anything.
    pub async fn spawn<F>(responder: F) -> Self
    where
        F: Fn(usize) -> Option<Vec<u8>> + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));

        let counter = hits.clone();
        let responder = Arc::new(responder);
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                let n = counter.fetch_add(1, Ordering::SeqCst);
                let responder = responder.clone();
                tokio::spawn(async move {
                    // Drain the request head; we never inspect it.
                    let mut buf = [0u8; 2048];
                    let _ = sock.read(&mut buf).await;
                    if let Some(resp) = responder(n) {
                        let _ = sock.write_all(&resp).await;
                        let _ = sock.flush().await;
                    }
                });
            }
        });

        Self {
            base_url: format!("http://{addr}"),
            hits,
        }
    }

    /// Number of connections accepted so far.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

/// Accept connections and then hold them open indefinitely without replying.
/// Models a black-holing server: the socket is live, so the client will wait
/// forever unless it enforces its own timeout.
pub async fn serve_never_responds() -> StubServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();

    tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            // Keep the socket alive so the peer sees an open, silent connection.
            held.push(sock);
        }
    });

    StubServer {
        base_url: format!("http://{addr}"),
        hits,
    }
}

/// Serve a response that declares a very large body and then dribbles it out
/// forever, so the client is always cut off by its own deadline rather than
/// reaching the end of the body.
///
/// Models a real speedtest endpoint on a fast line: the payload is larger than
/// one measurement window, so no single request ever completes.
pub async fn serve_endless_stream(chunk: usize, delay_ms: u64) -> StubServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            counter.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = sock.read(&mut buf).await;
                let head = "HTTP/1.1 200 OK\r\n\
                            Content-Type: application/octet-stream\r\n\
                            Content-Length: 1073741824\r\n\
                            \r\n";
                if sock.write_all(head.as_bytes()).await.is_err() {
                    return;
                }
                let payload = vec![b'x'; chunk];
                loop {
                    if sock.write_all(&payload).await.is_err() {
                        break;
                    }
                    if sock.flush().await.is_err() {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
            });
        }
    });

    StubServer {
        base_url: format!("http://{addr}"),
        hits,
    }
}

/// Assert that a live-API call degraded gracefully.
///
/// The contract (see the fix in the client's `get_json`): a non-JSON response
/// -- for example the HTML error page several `breitbandmessung.de` endpoints
/// currently return with HTTP 200 -- must surface as [`BbmError::Api`], never
/// as [`BbmError::Json`], which would mean the body reached the deserializer.
///
/// So a live test passes if the call parsed, or failed with any clean typed
/// error, and fails only on a JSON parse error. `on_ok` validates the happy
/// path when the endpoint is actually up.
pub fn assert_graceful<T>(result: crate::Result<T>, on_ok: impl FnOnce(T)) {
    match result {
        Ok(value) => on_ok(value),
        Err(crate::BbmError::Json(e)) => {
            panic!("a non-JSON response reached the deserializer: {e}")
        }
        Err(e) => eprintln!("endpoint unavailable, degraded gracefully: {e}"),
    }
}

/// Build a raw HTTP/1.1 response with an explicit `Content-Type`.
pub fn http_response(status: u16, content_type: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    )
    .into_bytes()
}
