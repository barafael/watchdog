use erro::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

mod erro;

#[cfg(test)]
mod test;

/// Signal for resetting the watchdog.
#[derive(Debug)]
pub struct Reset;

/// Signal on watchdog expire.
#[derive(Debug)]
pub struct Elapsed;

/// Watchdog holding the fixed duration.
pub struct Watchdog {
    /// The timeout interval.
    duration: Duration,
}

impl Watchdog {
    /// Make a watchdog with the given timeout duration.
    #[must_use]
    pub const fn with_timeout(duration: Duration) -> Self {
        Self { duration }
    }

    /// Spawn the watchdog actor.
    ///
    /// Returns the `reset_tx` and `elapsed_rx` needed for communicating with the watchdog.
    #[must_use]
    pub fn spawn(self) -> (mpsc::Sender<Reset>, oneshot::Receiver<Elapsed>) {
        let (reset_tx, reset_rx) = mpsc::channel(16);
        let (elapsed_tx, elapsed_rx) = oneshot::channel();
        tokio::spawn(self.run(reset_rx, elapsed_tx));
        (reset_tx, elapsed_rx)
    }

    /// Start the watchdog actor.
    async fn run(self, mut reset: mpsc::Receiver<Reset>, elapsed: oneshot::Sender<Elapsed>) {
        let sleep = tokio::time::sleep(self.duration);
        tokio::pin!(sleep);
        loop {
            tokio::select! {
                msg = reset.recv() => {
                    match msg {
                        Some(_) => sleep.as_mut().reset(Instant::now() + self.duration),
                        None => break,
                    }
                }
                _ = sleep.as_mut() => {
                    let _ = elapsed.send(Elapsed);
                    break;
                },
            }
        }
    }

    /// Reset the watchdog attached to `reset_tx`.
    ///
    /// # Errors
    ///
    /// If the watchdog is inactive, Err([`Error::Inactive`]) is returned.
    pub async fn reset(reset_tx: &mpsc::Sender<Reset>) -> Result<(), Error> {
        reset_tx.send(Reset).await.map_err(|_| Error::Inactive)
    }
}
