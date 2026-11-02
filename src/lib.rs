#![doc = include_str!("../README.md")]

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

    /// Run the watchdog event loop.
    ///
    /// Spawning is left to the caller: `tokio::spawn` it, `.await` it
    /// directly, or put it in a `JoinSet`. The loop exits when the sleep
    /// expires (after sending [`Expired`] on `expire_tx`) or when every
    /// `reset_tx` sender is dropped. It hands the watchdog back on exit, so
    /// restarting it means making new channels and running the loop again.
    pub async fn event_loop(
        self,
        mut reset_rx: mpsc::Receiver<Signal>,
        expire_tx: oneshot::Sender<Expired>,
    ) -> Self {
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
                () = sleep.as_mut(), if active => {
                    // on sleep expiry: use up `expire_tx`, then exit.
                    let _ = expire_tx.send(Expired);
                    break;
                },
            }
        }
        self
    }
}
