use crate::Signal;

use super::Watchdog;
use std::time::Duration;
use tokio::time::Instant;
use tokio_test::assert_elapsed;

#[tokio::test(start_paused = true)]
async fn watchdog_() {
    // Pre-conditions.
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));

    // Actions.
    let (watchdog, elapsed_rx) = watchdog.run();

    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        watchdog.send(Signal::Reset).await.unwrap();
    }

    // Let the watchdog expire.
    let now = Instant::now();
    elapsed_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(watchdog.send(Signal::Reset).await.is_err());
}
