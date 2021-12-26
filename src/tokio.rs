use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[cfg(unix)]
use tokio::signal::unix::SignalKind;

type BoxFuture<'a> = Pin<Box<dyn Future<Output = ()> + 'a>>;

pub fn depart<'a>() -> Departure<'a> {
    Departure {
        inner: Some(Inner::new()),
        fut: None,
    }
}

pub struct Departure<'a> {
    inner: Option<Inner<'a>>,
    fut: Option<BoxFuture<'a>>,
}

impl<'a> Departure<'a> {
    pub fn on_ctrl_c(mut self) -> Self {
        self.inner.as_mut().unwrap().on_ctrl_c();
        self
    }

    /// - catch ctrl+c + sigterm
    pub fn on_termination(mut self) -> Self {
        self.inner.as_mut().unwrap().on_termination();
        self
    }

    #[cfg(unix)]
    pub fn on_sigint(mut self) -> Self {
        self.inner.as_mut().unwrap().on_sigint();
        self
    }

    #[cfg(unix)]
    pub fn on_sigterm(mut self) -> Self {
        self.inner.as_mut().unwrap().on_sigterm();
        self
    }

    /// Catch a specific signal
    #[cfg(unix)]
    pub fn on_signal(mut self, signal: SignalKind) -> Self {
        self.inner.as_mut().unwrap().on_signal(signal);
        self
    }

    pub fn on_completion(mut self, fut: impl Future<Output = ()> + 'a) -> Self {
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

    fn on_completion(&mut self, fut: impl Future<Output = ()> + 'a) {
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
        drop(data_ref);

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
            let _ = crate::shutdown();
        });

        depart()
            .on_completion(async move {
                barrier.wait().await;
                std::future::pending::<()>().await;
            })
            .await;
    }
}
