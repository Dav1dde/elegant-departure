use crate::deplist::DepList;
use crate::signal::{Signal, WaitForSignalFuture};
use crate::BoxFuture;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Registry tracking all shutdown guards.
pub(crate) struct Registry {
    departures: DepList<Signal>,
    shutdown: Signal,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            departures: DepList::new(),
            shutdown: Signal::new(),
        }
    }

    pub fn create_guard(&mut self) -> ShutdownGuard {
        let done = Signal::new();

        self.departures.push(done.clone());

        ShutdownGuard {
            shutdown: self.shutdown.clone(),
            done,
        }
    }

    pub fn shutdown(&mut self) {
        self.shutdown.set();
    }

    pub fn wait_for_shutdown_complete(&self) -> BoxFuture<()> {
        let shutdown = self.shutdown.clone();
        // this retruns an iterator that clones the containing tokens on iteration
        // which also includes new items that get added to the collection while iterating
        let departures = self.departures.iter();

        Box::pin(async move {
            // wait until a shutdown is initiated
            shutdown.wait().await;

            // start consuming the done signals
            // TODO: we should allow a timeout here
            for signal in departures {
                signal.wait().await;
            }
        })
    }

    #[cfg(test)]
    pub fn reset(&mut self) {
        self.departures = DepList::new();
        self.shutdown = Signal::new();
    }
}

/// A guard which delays shutdown until it is cancelled or dropped.
///
/// The guard can only be created using [`get_shutdown_guard`](crate::get_shutdown_guard).
///
/// # Example:
///
/// ```no_run
/// # use tokio::select;
/// # async fn long_running_task() { }
/// # async fn cleanup() { }
/// # async fn fun() {
/// # let condition = true;
/// let guard = elegant_departure::get_shutdown_guard();
///
/// select! {
///     _ = guard.wait() => (),
///     _ = long_running_task() => (),
/// }
///
/// if condition {
///     guard.cancel();
/// }
///     
/// cleanup().await;
/// # }
/// ```
pub struct ShutdownGuard {
    shutdown: Signal,
    done: Signal,
}

impl ShutdownGuard {
    /// Returns a future which waits for the shutdown signal.
    ///
    /// Can be called multiple times.
    /// A cancelled guard can still be awaited for shutdown signal.
    pub fn wait(&self) -> WaitForShutdownFuture<'_> {
        WaitForShutdownFuture {
            inner: self.shutdown.wait(),
        }
    }

    /// Cancels the shutdown guard.
    ///
    /// After cancelling the guard, shutdown will no longer wait for the guard to be dropped.
    ///
    /// The guard can be cancelled multiple times, but this operation cannot be undone.
    pub fn cancel(&self) {
        self.done.set();
    }
}

impl<'a> std::fmt::Debug for ShutdownGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownGuard")
            .field("in_shutdown", &self.shutdown.is_set())
            .field("cancelled", &self.done.is_set())
            .finish()
    }
}

impl Drop for ShutdownGuard {
    /// Cancels the shutdown guard.
    fn drop(&mut self) {
        self.done.set();
    }
}

pin_project! {
    /// Future returned by the [`ShutdownGuard::wait`](ShutdownGuard::wait) method.
    pub struct WaitForShutdownFuture<'a> {
        #[pin]
        inner: WaitForSignalFuture<'a>,
    }
}

impl<'a> std::fmt::Debug for WaitForShutdownFuture<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitForShutdownFuture").finish()
    }
}

impl<'a> std::future::Future for WaitForShutdownFuture<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.inner.poll(cx)
    }
}
