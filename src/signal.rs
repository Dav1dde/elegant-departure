use pin_project_lite::pin_project;
use std::task::{Context, Poll};
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

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
            inner: self.token.cancelled(),
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
    pub struct WaitForSignalFuture<'a> {
        #[pin]
        inner: WaitForCancellationFuture<'a>,
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
        this.inner.poll(cx)
    }
}
