use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration, Instant};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
use clap::{Parser, ValueEnum};

#[cfg(test)]
mod test;

/// Signal for interacting with the watchdog.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cli", derive(ValueEnum, Parser))]
pub enum Signal {
    #[default]
    Reset,
    Stop,
}

/// Signal on watchdog expiration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Expired;

/// Watchdog holding the fixed duration.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    /// Returns the `reset_tx` and `expire_tx` channels needed for communicating with the watchdog.
    #[must_use]
    pub fn run(self) -> (mpsc::Sender<Signal>, oneshot::Receiver<Expired>) {
        let (reset_tx, reset_rx) = mpsc::channel(16);
        let (expire_tx, expire_rx) = oneshot::channel();
        tokio::spawn(self.event_loop(reset_rx, expire_tx));
        (reset_tx, expire_rx)
    }

    /// Run the watchdog event loop.
    async fn event_loop(
        self,
        mut reset_rx: mpsc::Receiver<Signal>,
        expire_tx: oneshot::Sender<Expired>,
    ) {
        let sleep = sleep(self.duration);
        tokio::pin!(sleep);
        let mut active = true;
        loop {
            tokio::select! {
                msg = reset_rx.recv() => {
                    match msg {
                        // on reset: set active, restart sleep.
                        Some(Signal::Reset) => {
                            active = true;
                            sleep.as_mut().reset(Instant::now() + self.duration);
                        }
                        // on stop: mark watchdog as not active.
                        Some(Signal::Stop) => active = false,
                        // on channel close: exit watchdog.
                        None => break,
                    }
                }
                _ = sleep.as_mut(), if active => {
                    // on sleep expiry: use up `expire_tx`, then exit.
                    let _ = expire_tx.send(Expired);
                    break;
                },
            }
        }
    }
}
