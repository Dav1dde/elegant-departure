use axum::{routing::get, Router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    println!("Listening on port 3000!");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(elegant_departure::tokio::depart().on_termination())
        .await?;

    Ok(())
}
