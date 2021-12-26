use lazy_static::lazy_static;
use pin_project_lite::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};

mod deplist;
mod signal;
#[cfg(feature = "tokio")]
pub mod tokio;

use self::deplist::DepList;
use self::signal::Signal;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}

struct Registry {
    departures: DepList<Signal>,
    shutdown: Signal,
}

impl Registry {
    fn new() -> Self {
        Registry {
            departures: DepList::new(),
            shutdown: Signal::new(),
        }
    }

    fn create_guard(&mut self) -> ShutdownGuard {
        let done = Signal::new();

        self.departures.push(done.clone());

        ShutdownGuard {
            shutdown: self.shutdown.clone(),
            done,
        }
    }

    fn shutdown(&mut self) {
        self.shutdown.set();
    }

    fn wait_for_shutdown_complete(&self) -> BoxFuture<()> {
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
    fn reset(&mut self) {
        self.departures = DepList::new();
        self.shutdown = Signal::new();
    }
}

pub struct ShutdownGuard {
    shutdown: Signal,
    done: Signal,
}

impl ShutdownGuard {
    pub fn wait(&self) -> WaitForShutdownFuture<'_> {
        WaitForShutdownFuture {
            inner: self.shutdown.wait(),
        }
    }

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
    fn drop(&mut self) {
        self.done.set();
    }
}

pin_project! {
    pub struct WaitForShutdownFuture<'a> {
        #[pin]
        inner: signal::WaitForSignalFuture<'a>,
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

#[cfg(test)]
pub(crate) fn reset() {
    REGISTRY.lock().unwrap().reset();
}

// private version that returns a BoxFuture, public version is a little bit more generic
pub(crate) fn private_wait_for_shutdown_complete() -> BoxFuture<()> {
    REGISTRY.lock().unwrap().wait_for_shutdown_complete()
}

pub fn get_shutdown_guard() -> ShutdownGuard {
    REGISTRY.lock().unwrap().create_guard()
}

pub fn wait_for_shutdown_complete() -> impl Future<Output = ()> + Send {
    REGISTRY.lock().unwrap().wait_for_shutdown_complete()
}

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
