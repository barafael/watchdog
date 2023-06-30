use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
use clap::ValueEnum;

#[cfg(test)]
mod test;

/// Signal for resetting the watchdog.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum Signal {
    #[default]
    Reset,
    Stop,
}

/// Signal on watchdog expire.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
    /// Returns the `watchdog` and `expiration` channels needed for communicating with the watchdog.
    #[must_use]
    pub fn run(self) -> (mpsc::Sender<Signal>, oneshot::Receiver<Expired>) {
        let (watchdog, watchdog_rx) = mpsc::channel(16);
        let (expiration_tx, expiration) = oneshot::channel();
        tokio::spawn(self.event_loop(watchdog_rx, expiration_tx));
        (watchdog, expiration)
    }

    /// Run the watchdog event loop. This should be allowed to run in parallel to avoid starvation.
    async fn event_loop(
        self,
        mut signal: mpsc::Receiver<Signal>,
        expired: oneshot::Sender<Expired>,
    ) {
        let sleep = tokio::time::sleep(self.duration);
        tokio::pin!(sleep);
        let mut active = true;
        loop {
            tokio::select! {
                msg = signal.recv() => {
                    match msg {
                        Some(Signal::Reset) => {
                            sleep.as_mut().reset(Instant::now() + self.duration);
                            active = true;
                        }
                        Some(Signal::Stop) => active = false,
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
