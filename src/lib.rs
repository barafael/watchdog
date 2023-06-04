use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

#[cfg(test)]
mod test;

/// Signal for resetting the watchdog.
#[derive(Debug)]
pub enum Reset {
    Start,
    Signal,
    Stop,
}

/// Signal on watchdog expire.
#[derive(Debug)]
pub struct Expired;

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
    /// Returns the `reset_tx` and `expired_rx` needed for communicating with the watchdog.
    #[must_use]
    pub fn spawn(self) -> (mpsc::Sender<Reset>, oneshot::Receiver<Expired>) {
        let (reset_tx, reset_rx) = mpsc::channel(16);
        let (expired_tx, expired_rx) = oneshot::channel();
        tokio::spawn(self.run(reset_rx, expired_tx));
        (reset_tx, expired_rx)
    }

    /// Start the watchdog actor.
    async fn run(self, mut reset: mpsc::Receiver<Reset>, expired: oneshot::Sender<Expired>) {
        let sleep = tokio::time::sleep(self.duration);
        tokio::pin!(sleep);
        let mut active = true;
        loop {
            tokio::select! {
                msg = reset.recv() => {
                    match msg {
                        Some(Reset::Start) => {
                            sleep.as_mut().reset(Instant::now() + self.duration);
                            active = true;
                        }
                        Some(Reset::Signal) =>{
                            sleep.as_mut().reset(Instant::now() + self.duration);
                            active=true;
                        }
                        Some(Reset::Stop) => {
                            active = false;
                        }
                        None => break,
                    }
                }
                _ = sleep.as_mut(), if active => {
                    let _ = expired.send(Expired);
                    break;
                },
            }
        }
    }
}
