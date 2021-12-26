use std::time::Duration;

use tokio::{signal::unix::SignalKind, time::sleep};

async fn worker(name: &'static str) {
    let guard = elegant_departure::get_shutdown_guard();

    println!("[{}] working", name);

    guard.wait().await;
    println!("[{}] shutting down", name);

    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("[{}] done", name);
}

#[tokio::main]
async fn main() {
    tokio::spawn(worker("worker 1"));
    tokio::spawn(worker("worker 2"));

    elegant_departure::tokio::depart()
        // Terminate on Ctrl+C and SIGTERM
        .on_termination()
        // Terminate on SIGUSR1
        .on_signal(SignalKind::user_defined1())
        // Automatically initiate a shutdown after 5 seconds
        .on_completion(sleep(Duration::from_secs(5)))
        .await
}
