// metrics.rs — Latency Tracking and Benchmark Statistics
// Logs per-request latency and can run a batch benchmark to measure average latency

use std::time::Instant;
use chrono::Local;

/// Holds metadata about a single completed request
#[derive(Debug, Clone)]
pub struct RequestRecord {
    pub method: String,
    pub path: String,
    pub latency_ms: u128,
    pub timestamp: String,
}

impl RequestRecord {
    /// Pretty-print the request record to the terminal
    pub fn log(&self) {
        let latency_label = format_latency(self.latency_ms);
        println!(
            "  ↳ {} {} — {} [{}]",
            self.method,
            self.path,
            latency_label,
            self.timestamp
        );
    }
}

/// Format latency with colour-coded label based on speed
fn format_latency(ms: u128) -> String {
    if ms < 50 {
        format!("{}ms ✅ fast", ms)
    } else if ms < 200 {
        format!("{}ms ⚡ ok", ms)
    } else {
        format!("{}ms ⚠️  slow", ms)
    }
}

/// Statistics collected over a benchmark run
#[derive(Debug, Default)]
pub struct BenchmarkStats {
    pub total_requests: usize,
    pub successful: usize,
    pub failed: usize,
    pub latencies_ms: Vec<u128>,
}

impl BenchmarkStats {
    pub fn record_success(&mut self, latency_ms: u128) {
        self.total_requests += 1;
        self.successful += 1;
        self.latencies_ms.push(latency_ms);
    }

    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed += 1;
    }

    pub fn average_latency(&self) -> f64 {
        if self.latencies_ms.is_empty() {
            return 0.0;
        }
        let sum: u128 = self.latencies_ms.iter().sum();
        sum as f64 / self.latencies_ms.len() as f64
    }

    pub fn min_latency(&self) -> u128 {
        *self.latencies_ms.iter().min().unwrap_or(&0)
    }

    pub fn max_latency(&self) -> u128 {
        *self.latencies_ms.iter().max().unwrap_or(&0)
    }

    pub fn p95_latency(&self) -> u128 {
        if self.latencies_ms.is_empty() {
            return 0;
        }
        let mut sorted = self.latencies_ms.clone();
        sorted.sort_unstable();
        // P95: value at 95th percentile (0-indexed, so idx = ceil(0.95 * n) - 1)
        let idx = (((sorted.len() as f64) * 0.95).ceil() as usize)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        sorted[idx]
    }

    pub fn print_report(&self) {
        println!("\n╔══════════════════════════════════════╗");
        println!("║       Benchmark Results               ║");
        println!("╠══════════════════════════════════════╣");
        println!("║  Total requests  : {:>6}             ║", self.total_requests);
        println!("║  Successful      : {:>6}             ║", self.successful);
        println!("║  Failed          : {:>6}             ║", self.failed);
        println!("╠══════════════════════════════════════╣");
        println!("║  Avg latency     : {:>6.1}ms          ║", self.average_latency());
        println!("║  Min latency     : {:>6}ms          ║", self.min_latency());
        println!("║  Max latency     : {:>6}ms          ║", self.max_latency());
        println!("║  P95 latency     : {:>6}ms          ║", self.p95_latency());
        println!("╚══════════════════════════════════════╝");
    }
}

/// Run benchmark mode — sends `count` HTTP requests to `target` and reports stats
pub async fn run_benchmark(target: &str, count: usize) {
    let mut stats = BenchmarkStats::default();

    for i in 1..=count {
        if i % 100 == 0 {
            println!("  Progress: {}/{} requests sent...", i, count);
        }

        let url = format!("{}/get", target.trim_end_matches('/'));
        let start = Instant::now();

        match send_benchmark_request(&url).await {
            Ok(_) => {
                let latency_ms = start.elapsed().as_millis();
                stats.record_success(latency_ms);
            }
            Err(e) => {
                eprintln!("  [FAIL] Request {}: {}", i, e);
                stats.record_failure();
            }
        }
    }

    stats.print_report();

    let throughput = if stats.average_latency() > 0.0 {
        1000.0 / stats.average_latency()
    } else {
        0.0
    };
    println!("\n  Estimated throughput: ~{:.0} requests/sec", throughput);
    println!("  Benchmark completed at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
}

/// Send a single HTTP GET request for benchmarking
async fn send_benchmark_request(url: &str) -> Result<(), String> {
    // Use hyper-util for a lightweight async GET request
    let client = hyper_util::client::legacy::Client::builder(
        hyper_util::rt::TokioExecutor::new(),
    )
    .build_http::<http_body_util::Empty<bytes::Bytes>>();

    let uri: hyper::Uri = url
        .parse()
        .map_err(|e| format!("Invalid URL '{}': {}", url, e))?;

    let req = hyper::Request::builder()
        .method("GET")
        .uri(uri)
        .header("User-Agent", "rust-http-proxy-benchmark/0.1")
        .body(http_body_util::Empty::new())
        .map_err(|e| format!("Build request error: {}", e))?;

    let response = client
        .request(req)
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    use http_body_util::BodyExt;
    response
        .into_body()
        .collect()
        .await
        .map_err(|e| format!("Read body error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_latency() {
        let mut stats = BenchmarkStats::default();
        stats.record_success(10);
        stats.record_success(20);
        stats.record_success(30);
        assert_eq!(stats.average_latency(), 20.0);
    }

    #[test]
    fn test_p95_latency() {
        let mut stats = BenchmarkStats::default();
        for i in 1..=100 {
            stats.record_success(i as u128);
        }
        assert_eq!(stats.p95_latency(), 95);
    }

    #[test]
    fn test_failed_requests() {
        let mut stats = BenchmarkStats::default();
        stats.record_success(50);
        stats.record_failure();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.failed, 1);
    }
}
