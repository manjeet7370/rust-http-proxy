mod parser;
mod proxy;
mod metrics;

use clap::Parser;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[command(name = "rust-http-proxy")]
#[command(about = "Simple HTTP reverse proxy")]
struct Args {
    // Backend server URL
    #[arg(long, default_value = "https://httpbin.org")]
    target: String,

    // Port to listen on
    #[arg(long, default_value_t = 8080)]
    port: u16,

    // Run benchmark mode
    #[arg(long)]
    benchmark: bool,

    // Number of benchmark requests
    #[arg(long, default_value_t = 1000)]
    bench_count: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.benchmark {
        println!("Running benchmark...");
        metrics::run_benchmark(&args.target, args.bench_count).await;
        return;
    }

    let addr = format!("127.0.0.1:{}", args.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind listener");

    println!("Listening on {}", addr);
    println!("Forwarding requests to {}\n", args.target);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                let target = args.target.clone();

                // Handle each client connection in a separate task
                tokio::spawn(async move {
                    proxy::handle_connection(socket, peer_addr, target).await;
                });
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
}
