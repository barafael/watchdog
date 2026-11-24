# Watchdog

Pretty simple but bulletproof watchdog actor.

Send reset signals on the mpsc sender at a fast enough rate, or else the expiration oneshot channel will trigger.

The watchdog is plain data. Its `event_loop` takes the channels as arguments and leaves the spawning to you: `tokio::spawn` it, `.await` it directly, or put it in a `JoinSet`. It exits when it expires or when every reset sender is dropped, and hands the watchdog back.

```rust
use simple_tokio_watchdog::{Signal, Watchdog};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

#[tokio::main]
async fn main() {
    let watchdog = Watchdog::with_timeout(Duration::from_millis(100));
    let (reset_tx, reset_rx) = mpsc::channel(16);
    let (expire_tx, mut expire_rx) = oneshot::channel();
    tokio::spawn(watchdog.event_loop(reset_rx, expire_tx));

    let mut duration = Duration::from_millis(4);
    loop {
        let sleep = tokio::time::sleep(duration);
        tokio::pin!(sleep);
        tokio::select! {
            _ = &mut expire_rx => {
                break;
            }
            () = sleep.as_mut() => {
                reset_tx.send(Signal::Reset).await.unwrap();
                duration *= 2;
                continue;
            }
        }
    }
    println!("{duration:?}");
}
```
