// proxy.rs — HTTP Forwarding Logic
// Reads the HTTP request from the client, forwards it to the backend, returns the response

use std::net::SocketAddr;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use chrono::Local;

use crate::parser;
use crate::metrics::RequestRecord;

/// Handle a single TCP client connection
/// Reads the HTTP request, forwards it, writes back the response
pub async fn handle_connection(mut socket: TcpStream, peer_addr: SocketAddr, target: String) {
    let mut buffer = vec![0u8; 8192]; // 8KB read buffer

    // Read incoming bytes from the client
    let n = match socket.read(&mut buffer).await {
        Ok(0) => {
            // Client closed connection with no data
            return;
        }
        Ok(n) => n,
        Err(e) => {
            eprintln!("[ERROR] Read from {}: {}", peer_addr, e);
            return;
        }
    };

    let raw_request = match std::str::from_utf8(&buffer[..n]) {
        Ok(s) => s.to_string(),
        Err(_) => {
            eprintln!("[ERROR] Non-UTF8 data from {}", peer_addr);
            let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\nInvalid UTF-8").await;
            return;
        }
    };

    // Parse the HTTP request
    let request = match parser::parse_request(&raw_request) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[ERROR] Parse error from {}: {}", peer_addr, e);
            let error_response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nBad Request: {}",
                e
            );
            let _ = socket.write_all(error_response.as_bytes()).await;
            return;
        }
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    println!(
        "[{}] {} {} {} → {}",
        timestamp,
        peer_addr,
        request.method,
        request.path,
        target
    );

    // Forward to backend and measure latency
    let start = Instant::now();

    let response_text = match forward_request(&target, &request).await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("[ERROR] Forward to backend failed: {}", e);
            format!(
                "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\n\r\nProxy Error: {}",
                e
            )
        }
    };

    let latency_ms = start.elapsed().as_millis();

    // Log the completed request with latency
    let record = RequestRecord {
        method: request.method.clone(),
        path: request.path.clone(),
        latency_ms,
        timestamp: Local::now().to_rfc3339(),
    };
    record.log();

    // Write the response back to the client
    if let Err(e) = socket.write_all(response_text.as_bytes()).await {
        eprintln!("[ERROR] Write to client {}: {}", peer_addr, e);
    }
}

/// Forward the parsed HTTP request to the backend server
/// Returns the raw HTTP response as a String
async fn forward_request(
    target: &str,
    request: &parser::HttpRequest,
) -> Result<String, String> {
    // Build the full URL
    let url = format!("{}{}", target.trim_end_matches('/'), request.path);

    // Use reqwest-style via hyper-util for HTTP/1.1 forwarding
    // We construct the request and send it using tokio's async HTTP client
    let client = build_http_client()?;

    let method = hyper::Method::from_bytes(request.method.as_bytes())
        .map_err(|e| format!("Invalid method: {}", e))?;

    let uri: hyper::Uri = url.parse().map_err(|e| format!("Invalid URL '{}': {}", url, e))?;

    let mut req_builder = hyper::Request::builder()
        .method(method)
        .uri(uri);

    // Forward headers from the original request
    for (key, value) in &request.headers {
        if key != "host" && key != "connection" {
            if let Ok(header_name) = hyper::header::HeaderName::from_bytes(key.as_bytes()) {
                if let Ok(header_value) = hyper::header::HeaderValue::from_str(value) {
                    req_builder = req_builder.header(header_name, header_value);
                }
            }
        }
    }

    let body_bytes = request.body.clone().unwrap_or_default();
    let hyper_body = http_body_util::Full::new(bytes::Bytes::from(body_bytes));
    let hyper_req = req_builder
        .body(hyper_body)
        .map_err(|e| format!("Build request error: {}", e))?;

    let response = client
        .request(hyper_req)
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = response.status();
    let mut response_text = format!(
        "HTTP/1.1 {} {}\r\n",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );

    // Copy response headers
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            response_text.push_str(&format!("{}: {}\r\n", name, v));
        }
    }
    response_text.push_str("\r\n");

    // Read response body
    use http_body_util::BodyExt;
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Read body error: {}", e))?
        .to_bytes();

    response_text.push_str(&String::from_utf8_lossy(&body_bytes));

    Ok(response_text)
}

/// Build a simple async HTTP/1.1 client using hyper-util
fn build_http_client() -> Result<
    hyper_util::client::legacy::Client<
        hyper_util::client::legacy::connect::HttpConnector,
        http_body_util::Full<bytes::Bytes>,
    >,
    String,
> {
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new(),
    )
    .build_http();
    Ok(client)
}
