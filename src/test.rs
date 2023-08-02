use crate::{Signal, Watchdog};

use tokio::{
    sync::oneshot::error::TryRecvError,
    time::{sleep, Instant},
};
use tokio_test::assert_elapsed;

use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn watchdog_works() {
    // Pre-conditions.
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));

    // Actions.
    let (reset_tx, expired_rx) = watchdog.run();

    for _ in 0..100 {
        sleep(Duration::from_millis(500)).await;
        reset_tx.send(Signal::Reset).await.unwrap();
    }

    // Let the watchdog expire.
    let now = Instant::now();
    expired_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(reset_tx.send(Signal::Reset).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn watchdog_stops_then_restarts() {
    // Pre-conditions.
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));

    // Actions.
    let (reset_tx, mut expired_rx) = watchdog.run();

    // sleep, then reset.
    sleep(Duration::from_millis(500)).await;
    reset_tx.send(Signal::Reset).await.unwrap();

    // sleep, then stop.
    sleep(Duration::from_millis(500)).await;
    reset_tx.send(Signal::Stop).await.unwrap();

    // sleep long, then assert there was no expiry.
    sleep(Duration::from_secs(5)).await;
    assert_eq!(Err(TryRecvError::Empty), expired_rx.try_recv());

    // Let the watchdog expire.
    let now = Instant::now();
    reset_tx.send(Signal::Reset).await.unwrap(); // resume
    expired_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(reset_tx.send(Signal::Reset).await.is_err());
}
