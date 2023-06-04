use crate::Reset;

use super::Watchdog;
use std::time::Duration;
use tokio::time::Instant;
use tokio_test::assert_elapsed;

#[tokio::test]
async fn spawn_watchdog() {
    // Pre-conditions.
    tokio::time::pause();
    let wdg = Watchdog::with_timeout(Duration::from_secs(1));

    // Actions.
    let (reset_tx, elapsed_rx) = wdg.spawn();

    for _ in 0..100 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        reset_tx.send(Reset::Signal).await.unwrap();
    }

    // Let the watchdog expire.
    let now = Instant::now();
    elapsed_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(matches!(reset_tx.send(Reset::Signal).await, Err(_)));
}
