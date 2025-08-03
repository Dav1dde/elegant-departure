use async_std::net::UdpSocket;
use async_std::task::{sleep, spawn};
use std::time::Duration;

async fn worker(name: &'static str) {
    let guard = elegant_departure::get_shutdown_guard();

    println!("[{name}] working");

    guard.wait().await;
    println!("[{name}] shutting down");

    sleep(Duration::from_secs(1)).await;
    println!("[{name}] done");
}

#[async_std::main]
async fn main() -> std::io::Result<()> {
    spawn(worker("worker 1"));
    spawn(worker("worker 2"));

    let socket = UdpSocket::bind("127.0.0.1:8000").await?;
    println!("Listening on {}", socket.local_addr()?);

    let mut buf = vec![0u8; 1024];
    loop {
        let (recv, peer) = socket.recv_from(&mut buf).await?;
        let sent = socket.send_to(&buf[..recv], &peer).await?;
        println!("Sent {sent} out of {recv} bytes to {peer}");

        if buf.starts_with(b"shutdown") {
            break;
        }
    }

    elegant_departure::shutdown().await;
    Ok(())
}
