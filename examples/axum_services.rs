use axum::{routing::get, Router};

#[derive(Debug, Clone)]
struct State {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = State {
        message: "Hello World".to_owned(),
    };

    tokio::spawn(http::start_server(state));
    tokio::spawn(database::start_cleanup_cron());
    tokio::spawn(utils::shutdown_notifier());

    // Initiates a shutdown on `Ctrl+C` or a `SIGTERM` signal and waits for all tasks to shutdown.
    elegant_departure::tokio::depart().on_termination().await;

    println!("Shutdown complete!");

    Ok(())
}

mod http {
    use std::time::Duration;

    use tokio::net::TcpListener;

    use super::*;

    pub async fn start_server(state: State) {
        // Grab a shutdown guard, it not only gives us a way to initiate a shutdown,
        // but also makes sure we do not exit, before all http requests have been served.
        let guard = elegant_departure::get_shutdown_guard()
            // This is an important service, and it can panic (notice the `unwrap`'s),
            // if that ever happens we want to make sure the service terminates.
            .shutdown_on_drop();

        println!("[web] Listening on port 3000!");
        let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
        let app = Router::new()
            .route("/", get(root))
            .route("/shutdown", get(shutdown))
            .with_state(state);

        axum::serve(listener, app)
            .with_graceful_shutdown(guard.wait_owned())
            .await
            .unwrap();

        println!("[web] All requests finished!");
    }

    async fn root(state: axum::extract::State<State>) -> String {
        state.0.message
    }

    async fn shutdown() {
        println!("[web] Someone requested a shutdown.");
        // Initiates a shutdown, but does not wait for it to complete.
        let _ = elegant_departure::shutdown();
        // Wait a bit to show that the server only stops once all requests finished.
        tokio::time::sleep(Duration::from_secs(2)).await;
        println!("[web] Shutdown initiated!")
    }
}

mod database {
    use std::time::Duration;

    pub async fn start_cleanup_cron() {
        let guard = elegant_departure::get_shutdown_guard()
            // We consider this task also critical, we should exit once this task crashes.
            .shutdown_on_drop();

        let mut interval = tokio::time::interval(Duration::from_secs(5));
        loop {
            // Wait for the next interval, or exit if we should shutdown.
            tokio::select! {
                _ = interval.tick() => (),
                _ = guard.wait() => break,
            };

            // This task is not interrupted by a shutdown signal and the shutdown will wait for it
            // to finish.
            println!("[database] Starting database cleanup ...");
            tokio::time::sleep(Duration::from_secs(3)).await;
            println!("[database] Database cleanup finished!");
        }

        println!("[database] Database cleanup exited!")
    }
}

mod utils {
    use std::time::Duration;

    /// An example task which does nothing except print when a shutdown has been initiated.
    pub async fn shutdown_notifier() {
        // The task is not critical, if it were to crash, it should not delay the shutdown.
        //  -> no `shutdown_on_drop`.
        //
        // We also do not need to keep the guard alive until the shutdown is completed.
        // But this means, it is possible that the task never runs to completion.
        elegant_departure::get_shutdown_guard().wait().await;

        println!("[shutdown] Shutdown initiated!");

        tokio::time::sleep(Duration::from_secs(10)).await;
        println!("[shutdown] I will never print");
    }
}
