use lazy_static::lazy_static;
use std::pin::Pin;
use std::sync::Mutex;
use futures::channel::oneshot;
use tokio_util::sync::CancellationToken;
use std::future::Future;


lazy_static! {
    static ref REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
}


#[derive(Debug)]
pub struct Cancelled;


struct Registry {
    departures: Option<Vec<oneshot::Receiver<()>>>,
    token: CancellationToken,
    in_shutdown: Option<CancellationToken>,
}

impl Registry {
    fn new() -> Self {
        Registry {
            departures: Some(Vec::new()),
            token: CancellationToken::new(),
            in_shutdown: None,
        }
    }

    fn create_guard(&mut self) -> Result<ShutdownGuard, Cancelled> {
        let departures = self.departures.as_mut().ok_or(Cancelled)?;

        let (sender, receiver) = oneshot::channel();

        departures.push(receiver);

        Ok(ShutdownGuard {
            _signal: Some(sender),
            token: self.token.clone(),
        })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ()>>> {
        if let Some(token) = &self.in_shutdown {
            let token = token.clone();
            return Box::pin(async move { token.cancelled().await });
        }

        self.token.cancel();

        let departures = match self.departures.take() {
            Some(departures) => departures,
            None => vec![],
        };

        // TODO: timeouts
        Box::pin(async move {
            for receiver in departures {
                match receiver.await {
                    Ok(_) => {},
                    Err(_) => {},
                }
            }
        })
    }
}


pub struct ShutdownGuard {
    _signal: Option<oneshot::Sender<()>>,
    token: CancellationToken,
}

impl ShutdownGuard {
    // TODO: impl Future for DepartureGuard
    pub async fn wait(&self) {
        self.token.cancelled().await
    }

    pub fn is_shutting_down(&self) -> bool {
        self.token.is_cancelled()
    }
}


pub async fn get_shutdown_guard() -> Result<ShutdownGuard, Cancelled> {
    REGISTRY.lock().unwrap().create_guard()
}


pub fn shutdown() -> impl Future<Output = ()> {
    REGISTRY.lock().unwrap().shutdown()
}
