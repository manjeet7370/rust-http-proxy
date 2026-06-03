use rust_http_proxy::metrics::{BenchmarkStats, RequestRecord};
use rust_http_proxy::parser;

#[test]
fn parse_get_request() {
    let raw = "GET /api/hello HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";

    let req = parser::parse_request(raw).unwrap();

    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/api/hello");
    assert_eq!(req.version, "HTTP/1.1");
    assert!(req.body.is_none());
}

#[test]
fn parse_post_request() {
    let raw = "POST /submit HTTP/1.1\r\nHost: localhost:8080\r\nContent-Length: 13\r\n\r\nHello, World!";

    let req = parser::parse_request(raw).unwrap();

    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/submit");
    assert_eq!(req.body.unwrap(), "Hello, World!");
}

#[test]
fn reject_bad_request() {
    let raw = "BADFORMAT\r\n\r\n";

    assert!(parser::parse_request(raw).is_err());
}

#[test]
fn header_names_are_case_insensitive() {
    let raw =
        "GET / HTTP/1.1\r\nContent-Type: application/json\r\nAccept: */*\r\n\r\n";

    let req = parser::parse_request(raw).unwrap();

    assert!(req.headers.contains_key("content-type"));
    assert_eq!(req.headers.get("accept").unwrap(), "*/*");
}

#[test]
fn empty_stats() {
    let stats = BenchmarkStats::default();

    assert_eq!(stats.average_latency(), 0.0);
    assert_eq!(stats.min_latency(), 0);
    assert_eq!(stats.max_latency(), 0);
}

#[test]
fn single_request_stats() {
    let mut stats = BenchmarkStats::default();

    stats.record_success(42);

    assert_eq!(stats.average_latency(), 42.0);
    assert_eq!(stats.min_latency(), 42);
    assert_eq!(stats.max_latency(), 42);
    assert_eq!(stats.successful, 1);
    assert_eq!(stats.failed, 0);
}

#[test]
fn multiple_request_stats() {
    let mut stats = BenchmarkStats::default();

    for value in [10, 20, 30, 40, 50] {
        stats.record_success(value);
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
fn request_log_should_not_crash() {
    let record = RequestRecord {
        method: "GET".into(),
        path: "/test".into(),
        latency_ms: 15,
        timestamp: "2024-01-01T00:00:00".into(),
    };

    record.log();
}