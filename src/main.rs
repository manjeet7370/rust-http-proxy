// main.rs — Entry point for the HTTP Reverse Proxy
// Starts the TCP listener on port 8080 and spawns async tasks per connection

mod parser;
mod proxy;
mod metrics;

use clap::Parser;
use tokio::net::TcpListener;

/// A high-performance async HTTP reverse proxy with latency tracking
#[derive(Parser, Debug)]
#[command(name = "rust-http-proxy")]
#[command(about = "HTTP reverse proxy with per-request latency logging")]
struct Args {
    /// Target backend URL to forward requests to
    #[arg(long, default_value = "https://httpbin.org")]
    target: String,

    /// Port to listen on
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Run benchmark mode: send N requests and print average latency
    #[arg(long)]
    benchmark: bool,

    /// Number of requests for benchmark mode
    #[arg(long, default_value_t = 1000)]
    bench_count: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.benchmark {
        println!("=== Benchmark Mode ===");
        println!("Target: {}", args.target);
        println!("Sending {} requests...\n", args.bench_count);
        metrics::run_benchmark(&args.target, args.bench_count).await;
        return;
    }

    let addr = format!("127.0.0.1:{}", args.port);
    let listener = TcpListener::bind(&addr).await.unwrap();

    println!("╔══════════════════════════════════════════╗");
    println!("║      rust-http-proxy  |  v0.1.0          ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Listening  : http://{}       ║", addr);
    println!("║  Forwarding : {}    ║", args.target);
    println!("║  Press Ctrl+C to stop                    ║");
    println!("╚══════════════════════════════════════════╝\n");

    // Accept incoming TCP connections in a loop
    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                let target = args.target.clone();
                // Spawn a new async task per connection — enables concurrency
                tokio::spawn(async move {
                    proxy::handle_connection(socket, peer_addr, target).await;
                });
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to accept connection: {}", e);
            }
        }
    }
}
