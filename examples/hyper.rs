use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use std::convert::Infallible;

async fn good_bye_world(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    // initiate a shutdown but don't wait for it to complete
    let _ = elegant_departure::shutdown();
    Ok(Response::new(Body::from("Good bye World!")))
}

async fn worker(name: &'static str) {
    let guard = elegant_departure::get_shutdown_guard();

    println!("[{}] working", name);

    guard.wait().await;
    println!("[{}] shutting down", name);

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    println!("[{}] done", name);
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        tokio::spawn(worker("worker 1"));

        // simulate a task that starts late or is dynamically added
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        tokio::spawn(worker("worker 2"));
    });

    let svc = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(good_bye_world)) });

    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr).serve(svc);

    println!("[hyper] listening on {}", addr);

    server
        .with_graceful_shutdown(elegant_departure::tokio::depart().on_ctrl_c())
        .await
        .unwrap();

    println!("[hyper] shutdown complete");
}
