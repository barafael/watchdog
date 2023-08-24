# Watchdog

Pretty simple but bulletproof watchdog actor.

Send reset signals on the mpsc sender at a fast enough rate, or else the expiration oneshot channel will trigger.

```rust
use tokio::select;
use simple_tokio_watchdog::{Signal, Watchdog};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let watchdog = Watchdog::with_timeout(Duration::from_millis(100));
    let (reset_tx, mut expired_rx) = watchdog.run();

    let mut duration = Duration::from_millis(4);
    loop {
        let sleep = tokio::time::sleep(duration);
        tokio::pin!(sleep);
        tokio::select! {
            _ = &mut expired_rx => {
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
