use pin_project_lite::pin_project;
use std::task::{Context, Poll};
use tokio_util::sync::{
    CancellationToken, WaitForCancellationFuture, WaitForCancellationFutureOwned,
};

#[derive(Clone)]
pub struct Signal {
    token: CancellationToken,
}

impl Signal {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    pub fn set(&self) {
        self.token.cancel();
    }

    pub fn is_set(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn wait(&self) -> WaitForSignalFuture<'_> {
        WaitForSignalFuture {
            inner: WaitForSignal::Borrowed {
                f: self.token.cancelled(),
            },
        }
    }

    pub fn wait_owned(&self) -> WaitForSignalFuture<'static> {
        WaitForSignalFuture {
            inner: WaitForSignal::Owned {
                f: self.token.clone().cancelled_owned(),
            },
        }
    }
}

impl Default for Signal {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal")
            .field("is_set", &self.is_set())
            .finish()
    }
}

pin_project! {
    #[project = WaitForSignalProj]
    enum WaitForSignal<'a> {
        Borrowed { #[pin] f: WaitForCancellationFuture<'a> },
        Owned { #[pin] f: WaitForCancellationFutureOwned },
    }
}

pin_project! {
    pub struct WaitForSignalFuture<'a> {
        #[pin]
        inner: WaitForSignal<'a>,
    }
}

impl<'a> std::fmt::Debug for WaitForSignalFuture<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WaitForSignalFuture").finish()
    }
}

impl<'a> std::future::Future for WaitForSignalFuture<'a> {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        match this.inner.project() {
            WaitForSignalProj::Borrowed { f } => f.poll(cx),
            WaitForSignalProj::Owned { f } => f.poll(cx),
        }
    }
}
