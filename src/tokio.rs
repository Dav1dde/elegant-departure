//! Additional tokio integration, for builtin shutdown on signal, ctrl-c, etc.
//!
//! # Example:
//!
//! ```rust
//! # use tokio::signal::unix::SignalKind;
//! # use tokio::time::sleep;
//! # use std::time::Duration;
//! # async fn worker(_: &'static str) {}
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
//!         .on_completion(sleep(Duration::from_secs(5)))
//!         .await
//! }
//! ```

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

type BoxFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Creates the [`Departure`] future builder to set up and customize shutdown conditions.
///
/// Returns the [`Departure`] builder. Refer to the [`Departure`] documentation for customization
/// options.
///
/// # Example:
///
/// ```no_run
/// # async fn fun() {
/// elegant_departure::tokio::depart()
///     .on_termination()
///     .await
/// # }
/// ```
pub fn depart<'a>() -> Departure<'a> {
    Departure {
        inner: Some(Inner::new()),
        fut: None,
    }
}

/// Future builder for customizing shutdown conditions.
///
/// Can be created using the [`depart`] function.
///
/// # Panics
///
/// The returned [future](Departure) panics when it is awaited without at least calling one builder
/// method, to prevent misuse.
///
/// If you just want to await shutdown completion, use
/// [`wait_for_shutdown_complete`](crate::wait_for_shutdown_complete) instead.
///
/// # Examples:
///
/// The most common usecase, to automatically trigger a shutdown on `Ctrl+C` and `SIGTERM` (on
/// unix platforms).
///
/// ```no_run
/// # async fn fun() {
/// elegant_departure::tokio::depart()
///     .on_termination()
///     .await
/// # }
/// ```
///
/// Shutdown on termination or after 5 seconds (a custom condition):
///
/// ```no_run
/// # async fn fun() {
/// elegant_departure::tokio::depart()
///     .on_termination()
///     // Initiates a shutdown when the future completes (after 5 seconds)
///     .on_completion(tokio::time::sleep(std::time::Duration::from_secs(5)))
///     .await
/// # }
/// ```
pub struct Departure<'a> {
    inner: Option<Inner<'a>>,
    fut: Option<BoxFuture<'a>>,
}

impl<'a> Departure<'a> {
    /// Initiates a shutdown on `Ctrl+C`.
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     .on_ctrl_c()
    ///     .await
    /// # }
    /// ```
    pub fn on_ctrl_c(mut self) -> Self {
        self.inner.as_mut().unwrap().on_ctrl_c();
        self
    }

    /// Initiates a shutdown on `Ctrl+C` and `SITGERM` (on unix).
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     .on_termination()
    ///     .await
    /// # }
    /// ```
    pub fn on_termination(mut self) -> Self {
        self.inner.as_mut().unwrap().on_termination();
        self
    }

    /// Initiates a shutdown on `SIGINT`.
    ///
    /// Convenience method for [`on_signal(SignalKind::interrupt())`](Departure::on_signal).
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     .on_sigint()
    ///     .await
    /// # }
    /// ```
    #[cfg(any(unix, doc))]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn on_sigint(mut self) -> Self {
        self.inner.as_mut().unwrap().on_sigint();
        self
    }

    /// Initiates a shutdown on `SIGTERM`.
    ///
    /// Convenience method for [`on_signal(SignalKind::terminate())`](Departure::on_signal).
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     .on_sigterm()
    ///     .await
    /// # }
    /// ```
    #[cfg(any(unix, doc))]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn on_sigterm(mut self) -> Self {
        self.inner.as_mut().unwrap().on_sigterm();
        self
    }

    /// Initiates a shutdown on a specific signal.
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     .on_signal(tokio::signal::unix::SignalKind::user_defined1())
    ///     .await
    /// # }
    /// ```
    #[cfg(any(unix, doc))]
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    pub fn on_signal(mut self, signal: SignalKind) -> Self {
        self.inner.as_mut().unwrap().on_signal(signal);
        self
    }

    /// Initiates a shutdown when the passed future completes.
    ///
    /// ```no_run
    /// # async fn fun() {
    /// elegant_departure::tokio::depart()
    ///     // Shutdown after 5 seconds
    ///     .on_completion(tokio::time::sleep(std::time::Duration::from_secs(5)))
    ///     .await
    /// # }
    /// ```
    pub fn on_completion(mut self, fut: impl Future<Output = ()> + Send + 'a) -> Self {
        self.inner.as_mut().unwrap().on_completion(fut);
        self
    }
}

impl<'a> Future for Departure<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.fut.is_none() {
            self.fut = Some(self.inner.take().unwrap().into_future());
        }

        self.fut.as_mut().unwrap().as_mut().poll(cx)
    }
}

struct Inner<'a> {
    futures: Vec<BoxFuture<'a>>,
}

impl<'a> Inner<'a> {
    fn new() -> Self {
        Self {
            futures: vec![crate::private_wait_for_shutdown_complete()],
        }
    }

    fn on_termination(&mut self) {
        self.on_ctrl_c();
        #[cfg(unix)]
        self.on_signal(SignalKind::terminate());
    }

    fn on_ctrl_c(&mut self) {
        self.futures.push(Box::pin(async {
            let _ = tokio::signal::ctrl_c().await;
        }));
    }

    #[cfg(unix)]
    fn on_sigint(&mut self) {
        self.on_signal(SignalKind::interrupt());
    }

    #[cfg(unix)]
    fn on_sigterm(&mut self) {
        self.on_signal(SignalKind::terminate());
    }

    #[cfg(unix)]
    fn on_signal(&mut self, signal: SignalKind) {
        let mut s = tokio::signal::unix::signal(signal).unwrap();
        self.futures.push(Box::pin(async move {
            let _ = s.recv().await;
        }));
    }

    fn on_completion(&mut self, fut: impl Future<Output = ()> + Send + 'a) {
        self.futures.push(Box::pin(fut));
    }

    fn into_future(self) -> BoxFuture<'a> {
        // If there is only one future (the "wait for shutdown" future),
        // panic to indicate invalid usage.
        if self.futures.len() == 1 {
            panic!(
                "invalid usage of `elegant_departure::tokio::depart()` \
                   choose at least one termination condition, e.g. \
                   `elegant_departure::tokio::depart().on_termination().await`."
            );
        }

        Box::pin(async move {
            futures::future::select_all(self.futures).await;
            crate::shutdown().await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn ensure_send() {
        fn is_send<T: Send>(_: T) {}
        is_send(depart())
    }

    #[tokio::test]
    #[serial]
    async fn should_allow_non_static_futures() {
        crate::reset();

        let mut data = "foo".to_owned();

        let data_ref = &mut data;
        depart()
            .on_completion(async {
                data_ref.push_str("bar");
            })
            .await;

        assert_eq!("foobar", data);
    }

    #[tokio::test]
    #[serial]
    async fn should_complete_on_shutdown() {
        crate::reset();

        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));

        let b1 = barrier.clone();
        tokio::spawn(async move {
            b1.wait().await;
            drop(crate::shutdown());
        });

        depart()
            .on_completion(async move {
                barrier.wait().await;
                std::future::pending::<()>().await;
            })
            .await;
    }
}
