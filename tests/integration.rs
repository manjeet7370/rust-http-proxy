// tests/integration.rs — Integration Tests
// Tests end-to-end behavior of the proxy components

#[cfg(test)]
mod integration_tests {
    use rust_http_proxy::parser;
    use rust_http_proxy::metrics::BenchmarkStats;

    // ─── Parser Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_parse_valid_get() {
        let raw = "GET /api/hello HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let req = parser::parse_request(raw).expect("Should parse valid GET");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/hello");
        assert_eq!(req.version, "HTTP/1.1");
        assert!(req.body.is_none());
    }

    #[test]
    fn test_parse_valid_post_with_body() {
        let raw = "POST /submit HTTP/1.1\r\nHost: localhost:8080\r\nContent-Length: 13\r\n\r\nHello, World!";
        let req = parser::parse_request(raw).expect("Should parse valid POST");
        assert_eq!(req.method, "POST");
        assert_eq!(req.path, "/submit");
        assert!(req.body.is_some());
        assert_eq!(req.body.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_parse_missing_path() {
        let raw = "BADFORMAT\r\n\r\n";
        let result = parser::parse_request(raw);
        assert!(result.is_err(), "Should fail on malformed request line");
    }

    #[test]
    fn test_parse_headers_case_insensitive() {
        let raw = "GET / HTTP/1.1\r\nContent-Type: application/json\r\nAccept: */*\r\n\r\n";
        let req = parser::parse_request(raw).unwrap();
        // Headers are lowercased internally
        assert!(req.headers.contains_key("content-type"));
        assert_eq!(req.headers["accept"], "*/*");
    }

    // ─── Metrics Tests ───────────────────────────────────────────────────────

    #[test]
    fn test_benchmark_stats_empty() {
        let stats = BenchmarkStats::default();
        assert_eq!(stats.average_latency(), 0.0);
        assert_eq!(stats.min_latency(), 0);
        assert_eq!(stats.max_latency(), 0);
    }

    #[test]
    fn test_benchmark_stats_single() {
        let mut stats = BenchmarkStats::default();
        stats.record_success(42);
        assert_eq!(stats.average_latency(), 42.0);
        assert_eq!(stats.min_latency(), 42);
        assert_eq!(stats.max_latency(), 42);
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_benchmark_stats_mixed() {
        let mut stats = BenchmarkStats::default();
        for latency in [10, 20, 30, 40, 50] {
            stats.record_success(latency);
        }
        stats.record_failure();
        assert_eq!(stats.total_requests, 6);
        assert_eq!(stats.successful, 5);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.average_latency(), 30.0);
        assert_eq!(stats.min_latency(), 10);
        assert_eq!(stats.max_latency(), 50);
    }

    #[test]
    fn test_request_record_log_does_not_panic() {
        let record = rust_http_proxy::metrics::RequestRecord {
            method: "GET".to_string(),
            path: "/test".to_string(),
            latency_ms: 15,
            timestamp: "2024-01-01T00:00:00".to_string(),
        };
        // Should not panic
        record.log();
    }
}
