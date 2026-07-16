use axum::http::StatusCode;
use ldpc_rust::server_router::router;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

#[tokio::main]
async fn main() {
    // Hard wall-clock cap per request so a stuck/slow decode (e.g. an SPA
    // request sitting at MAX_ITERATIONS) can't tie up a worker
    // indefinitely. Tune based on real p99 latency once you have
    // production numbers - this is a defensive ceiling, not a target.
    let app = router().layer(TimeoutLayer::with_status_code(
        StatusCode::GATEWAY_TIMEOUT,
        Duration::from_secs(10),
    ));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {addr}: {e}");
            std::process::exit(1);
        }
    };

    println!("LDPC microservice running on {addr}");

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}

/// Waits for Ctrl+C or SIGTERM (the signal Docker sends on `docker stop`)
/// so in-flight requests can finish instead of being dropped mid-response.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("shutdown signal received, draining in-flight requests");
}
