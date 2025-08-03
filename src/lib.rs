//! Runtime independent async graceful shutdowns.
//!
//! Provides an easy meachnism to initiate a shutdown from anywhere
//! and to wait for all workers and tasks to gracefully finish.
//!
//! Why you would want to use `elegant-departure`:
//! - Easy to use and minimal API
//! - Runtime independent (works with `tokio`, `async-std`, `smol`, ...)
//! - Additional integrations for `tokio` (shutdown on `ctrl-c`, signals etc.)
//!
//! # Usage
//!
//! This crate is [on crates.io](https://crates.io/crates/elegant-departure) and can be
//! used by adding it to your dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! elegant-departure = "0.3"
//! ```
//!
//! For a optional tokio integration, you need to enable the tokio feature:
//!
//! ```toml
//! [dependencies]
//! elegant-departure = { version = "0.3", features = "tokio" }
//! ```
//!
//! # Example: Axum
//!
//! Axum is easily integrated through the `tokio` integration.
//!
//! ```no_run
//! use axum::{routing::get, Router};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = Router::new().route("/", get(|| async { "Hello, World!" }));
//!
//!     println!("Listening on port 3000!");
//!     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
//!
//!     axum::serve(listener, app)
//!         .with_graceful_shutdown(elegant_departure::tokio::depart().on_termination())
//!         .await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! The [Axum Services] example sets up `axum` and multiple other background tasks/services:
//!
//! ```no_run
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     tokio::spawn(http::start_server());
//!     tokio::spawn(database::start_cleanup_cron());
//!     tokio::spawn(utils::shutdown_notifier());
//!
//!     // Initiates a shutdown on `Ctrl+C` or a `SIGTERM` signal and waits for all tasks to shutdown.
//!     elegant_departure::tokio::depart().on_termination().await;
//!
//!     println!("Shutdown complete!");
//!
//!     Ok(())
//! }
//! # mod http { pub async fn start_server() {} }
//! # mod database { pub async fn start_cleanup_cron() {} }
//! # mod utils { pub async fn shutdown_notifier() {} }
//! ```
//!
//! # Example: Simple worker
//!
//! A minimal example with multiple workers getting notified on shutdown.
//! The workers need an additional second after notification to exit.
//!
//! ```no_run
//! async fn worker(name: &'static str) {
//!     // Creates a new shutdown guard, the shutdown will wait for all guards to either be dropped
//!     // or explicitly cancelled.
//!     let guard = elegant_departure::get_shutdown_guard();
//!
//!     println!("[{}] working", name);
//!
//!     // The future completes when a shutdown is initiated
//!     guard.wait().await;
//!
//!     println!("[{}] shutting down", name);
//!     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//!     println!("[{}] done", name);
//!     // Guard dropped here, signalling shutdown completion
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     tokio::spawn(worker("worker 1"));
//!     tokio::spawn(worker("worker 2"));
//!
//!     // Could be any condition, e.g. waiting for a HTTP request
//!     tokio::signal::ctrl_c().await.unwrap();
//!     // Initiates the shutdown and waits for all tasks to complete
//!     // Note: you could wrap this future using `tokio::time::timeout`.
//!     elegant_departure::shutdown().await;
//!
//!     println!("Shutdown completed");
//! }
//! ```
//!
//! Example output:
//!
//! ```txt
//! [worker 1] working
//! [worker 2] working
//! ^C
//! [worker 1] shutting down
//! [worker 2] shutting down
//! [worker 2] done
//! [worker 1] done
//! Shutdown completed
//! ```
//!
//! # Example: Tokio integration
//!
//! The same example as before, but this time using the tokio integration to exit
//! on a termination signal (`Ctrl+C` or `SIGTERM`), `SIGUSR1` or a custom future,
//! whichever happens first.
//!
//! ```no_run
//! # #[cfg(feature = "tokio")] mod doc {
//! use tokio::{signal::unix::SignalKind, time::sleep};
//!
//! /* ... */
//! # async fn worker(_: &'static str) {}
//!
//! #[tokio::main]
//! async fn main() {
//!     tokio::spawn(worker("worker 1"));
//!     tokio::spawn(worker("worker 2"));
//!
//!     elegant_departure::tokio::depart()
//!         // Terminate on Ctrl+C and SIGTERM
//!         .on_termination()
//!         // Terminate on SIGUSR1
//!         .on_signal(SignalKind::user_defined1())
//!         // Automatically initiate a shutdown after 5 seconds
//!         .on_completion(sleep(std::time::Duration::from_secs(5)))
//!         .await
//! }
//! # }
//! ```
//!
//! # More Examples
//!
//! More examples can be found in the [examples] directory of the source code repository:
//!
//! - [Simple]: the full simple example from above
//! - [Axum]: the full axum example from above
//! - [Axum Services]: a bigger axum example, which also starts multiple background services
//! - [Tokio]: the full tokio example from above
//! - [Worker]: example implementation of a worker using `select!`
//! - [Smol]: example using the smol runtime
//! - [Async Std]: example using the async_std runtime
//!
//! [examples]: https://github.com/Dav1dde/elegant-departure/tree/master/examples
//! [Simple]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/simple.rs
//! [Axum]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/axum.rs
//! [Axum Services]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/axum_services.rs
//! [Tokio]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/tokio.rs
//! [Worker]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/worker.rs
//! [Smol]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/smol.rs
//! [Async Std]: https://github.com/Dav1dde/elegant-departure/tree/master/examples/async_std.rs
//!
//! # Things to consider
//!
//! - **Do not** wait for a shutdown while holding a shutdown guard, this will wait forever:
//!
//! ```no_run
//! # async fn fun() {
//! let guard = elegant_departure::get_shutdown_guard();
//! // This will never exit, because `guard` is never dropped/cancelled!
//! elegant_departure::shutdown().await;
//! # }
//! ```
//!
//! - **Do not** dynamically allocate shutdown guards for short living tasks.
//!   Currently every shutdown guard is  stored globally and never freed.
//!   If you dynamically allocate shutdown guards, this essentially leaks memory.
//!
#![cfg_attr(docsrs, feature(doc_cfg))]

mod deplist;
mod registry;
mod signal;

#[cfg(feature = "tokio")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
pub mod tokio;

pub use registry::{ShutdownGuard, WaitForShutdownFuture};

use lazy_static::lazy_static;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

lazy_static! {
    static ref REGISTRY: Mutex<registry::Registry> = Mutex::new(registry::Registry::new());
}

pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[cfg(test)]
pub(crate) fn reset() {
    REGISTRY.lock().unwrap().reset();
}

// private version that returns a boxed future, public version is a little bit more generic
#[cfg(feature = "tokio")]
pub(crate) fn private_wait_for_shutdown_complete() -> BoxFuture<()> {
    REGISTRY.lock().unwrap().wait_for_shutdown_complete()
}

/// Creates a new shutdown guard and returns it.
///
/// A shutdown guard is a handle which can be cancelled
/// and is automatically cancelled when dropped.
/// After initiating the shutdown the guard's future will complete
/// to indicate that the shutdown has been requested.
///
/// The function holding the guard should start gracefully shutting
/// down (stop accepting new data, save state, etc.) then exit.
///
/// **Note:** Do not dynamically create shutdown guards for short living taks.
/// A global registry remembers every guard, which may lead to a memory leak
/// when continuosly creating and destryoing guards.
///
/// # Example:
///
/// ```no_run
/// use futures::FutureExt;
/// # async fn do_work(name: u64) -> u64 { name }
/// async fn worker(name: u64) {
///     let guard = elegant_departure::get_shutdown_guard();
///
///     let mut counter = 0;
///     loop {
///         // do some work and wait for the shutdown
///         let result = futures::select! {
///             _ = guard.wait().fuse() => None,
///             r = do_work(name).fuse() => Some(r),
///         };
///
///         let result = match result {
///             None => break, // shutdown received, terminate the worker
///             Some(x) => x,
///         };
///
///         counter += result;
///         println!("[worker {}] Did some hard work", name);
///     }
///
///     println!("[worker {}] Created {} work units, cleaning up", name, counter);
///     // Do any additional cleanup that is necessary, flushing state etc.
///     println!("[worker {}] Done", name);
/// }
/// ```
pub fn get_shutdown_guard() -> ShutdownGuard {
    REGISTRY.lock().unwrap().create_guard()
}

/// Creates a future which terminates when the shutdown is completed.
///
/// A shutdown is complete when a shutdown was initated and all guards have been dropped or
/// cancelled.
///
/// This function is usually not required. Typically you would await
/// the future returned by [shutdown] instead or use a [guard](get_shutdown_guard).
pub fn wait_for_shutdown_complete() -> impl Future<Output = ()> + Send {
    REGISTRY.lock().unwrap().wait_for_shutdown_complete()
}

/// Initiates the global shutdown.
///
/// Triggers all shutdown guards and returns a future which resolves once all processing has
/// stopped (all guards have been dropped or cancelled).
///
/// The returned future does not need to be awaited, this is useful e.g. when initiating the
/// shutdown from a HTTP handler or some other external event.
///
/// # Examples:
///
/// Shutdown initiated in main after a condition (`ctrl+c`):
///
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     /* spawn workers, handle requests, etc. */
///
///     // Could be any condition, e.g. waiting for a HTTP request
///     tokio::signal::ctrl_c().await.unwrap();
///
///     // Initiate shutdown and wait for it to complete before exiting the program.
///     // Note: you could wrap this future using `tokio::time::timeout`.
///     elegant_departure::shutdown().await;
/// }
/// ```
pub fn shutdown() -> impl Future<Output = ()> + Send {
    REGISTRY.lock().unwrap().shutdown();
    REGISTRY.lock().unwrap().wait_for_shutdown_complete()
}

#[cfg(test)]
mod tests {
    use super::{get_shutdown_guard, shutdown, wait_for_shutdown_complete};
    use crate::signal::Signal;
    use serial_test::serial;
    use std::{sync::Arc, time::Duration};
    use tokio::sync::Barrier;

    #[tokio::test]
    #[serial]
    async fn shutdown_api() {
        crate::reset();

        let b1 = Arc::new(Barrier::new(3));
        let b2 = b1.clone();
        let b3 = b1.clone();

        let s10 = Signal::new();
        let s11 = s10.clone();
        let s20 = Signal::new();
        let s21 = s20.clone();

        let cont = Signal::new();
        let cont1 = cont.clone();

        let worker1 = tokio::spawn(async move {
            let guard = get_shutdown_guard();
            let _ = tokio::join!(guard.wait(), b2.wait());
            s11.set();
            cont1.wait().await;
        });

        let worker2 = tokio::spawn(async move {
            let _ = tokio::join!(wait_for_shutdown_complete(), b3.wait());
            s21.set();
        });

        // worker that never completes, but cancels the guard
        let _worker3 = tokio::spawn(async {
            let guard = get_shutdown_guard();
            guard.cancel();
            std::future::pending::<()>().await;
        });

        b1.wait().await;
        assert!(!s10.is_set());
        assert!(!s20.is_set());

        // initiate shutdown, worker 1 shutdown guard opens and sets s10
        let fut = shutdown();
        tokio::pin!(fut);

        s10.wait().await;
        assert!(s10.is_set());
        assert!(!s20.is_set());

        // make sure we're sill marked as shutdown in progress and the future does not complete
        // 5 times just for good measure
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(0)).await;
            assert!(matches!(futures::poll!(&mut fut), std::task::Poll::Pending));
        }
        assert!(!s20.is_set());

        // let the workers finish
        cont.set();

        // worker should exit any moment now -> shutdown should complete
        fut.await;

        worker1.await.unwrap();
        worker2.await.unwrap();

        // both workers have completed and set the signal
        assert!(s10.is_set());
        assert!(s20.is_set());
    }
}
