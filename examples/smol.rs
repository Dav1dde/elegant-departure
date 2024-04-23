use anyhow::Result;
use smol::io::AsyncReadExt;
use smol::{future, Async};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

async fn serve(mut stream: Async<TcpStream>) -> Result<()> {
    println!("Serving http://{}", stream.get_ref().local_addr()?);

    let mut buffer = [0u8; 13];
    stream.read_exact(&mut buffer).await?;
    if &buffer == b"GET /shutdown" {
        println!("Initiating shutdown!");
        drop(elegant_departure::shutdown());
    }

    Ok(())
}

async fn listen(listener: Async<TcpListener>) -> Result<()> {
    println!("Listening on http://{}", listener.get_ref().local_addr()?);

    loop {
        // We could also race on a shutdown guard here to stop accepting connections
        // and implement graceful shutdown behaviour for the http server.
        let (stream, _) = listener.accept().await?;

        smol::spawn(async move {
            if let Err(err) = serve(stream).await {
                println!("Connection error: {:#?}", err);
            }
        })
        .detach();
    }
}

async fn worker(name: &'static str) {
    let guard = elegant_departure::get_shutdown_guard();

    println!("[{}] working", name);

    guard.wait().await;
    println!("[{}] shutting down", name);

    smol::Timer::after(Duration::from_secs(1)).await;
    println!("[{}] done", name);
}

fn main() -> Result<()> {
    smol::block_on(async_main())
}

async fn async_main() -> Result<()> {
    smol::spawn(worker("worker 1")).detach();
    smol::spawn(worker("worker 2")).detach();

    let http = listen(Async::<TcpListener>::bind(([127, 0, 0, 1], 8000))?);
    let shutdown = async {
        elegant_departure::wait_for_shutdown_complete().await;
        Ok(())
    };
    future::race(shutdown, http).await?;

    Ok(())
}
