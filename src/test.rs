use crate::{Expired, Signal, Watchdog};

use tokio::{
    sync::{
        mpsc,
        oneshot::{self, error::TryRecvError},
    },
    time::{sleep, Instant},
};
use tokio_test::assert_elapsed;

use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn watchdog_works() {
    // Pre-conditions.
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));
    let (reset_tx, reset_rx) = mpsc::channel(16);
    let (expire_tx, expire_rx) = oneshot::channel();

    // Actions.
    tokio::spawn(watchdog.event_loop(reset_rx, expire_tx));

    for _ in 0..100 {
        sleep(Duration::from_millis(500)).await;
        reset_tx.send(Signal::Reset).await.unwrap();
    }

    // Let the watchdog expire.
    let now = Instant::now();
    expire_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(reset_tx.send(Signal::Reset).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn watchdog_stops_then_restarts() {
    // Pre-conditions.
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));
    let (reset_tx, reset_rx) = mpsc::channel(16);
    let (expire_tx, mut expire_rx) = oneshot::channel();

    // Actions.
    tokio::spawn(watchdog.event_loop(reset_rx, expire_tx));

    // sleep, then reset.
    sleep(Duration::from_millis(500)).await;
    reset_tx.send(Signal::Reset).await.unwrap();

    // sleep, then stop.
    sleep(Duration::from_millis(500)).await;
    reset_tx.send(Signal::Stop).await.unwrap();

    // sleep long, then assert there was no expiry.
    sleep(Duration::from_secs(5)).await;
    assert_eq!(Err(TryRecvError::Empty), expire_rx.try_recv());

    // Let the watchdog expire.
    let now = Instant::now();
    reset_tx.send(Signal::Reset).await.unwrap(); // resume
    expire_rx.await.unwrap();

    // Post conditions.
    assert_elapsed!(now, Duration::from_secs(1));

    assert!(reset_tx.send(Signal::Reset).await.is_err());
}

/// No spawn: fill the channel by hand, drop the sender, await the loop.
#[tokio::test(start_paused = true)]
async fn exits_without_expiring_when_every_sender_is_dropped() {
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));
    let (reset_tx, reset_rx) = mpsc::channel(16);
    let (expire_tx, expire_rx) = oneshot::channel();

    reset_tx.send(Signal::Reset).await.unwrap();
    reset_tx.send(Signal::Stop).await.unwrap();
    drop(reset_tx);

    let now = Instant::now();
    let returned = watchdog.event_loop(reset_rx, expire_tx).await;

    // `recv()` said `None` straight after the queued signals: no time passed.
    assert_elapsed!(now, Duration::ZERO);
    // The loop hands the watchdog back.
    assert_eq!(returned, watchdog);
    // `expire_tx` was dropped unused.
    assert!(expire_rx.await.is_err());
}

/// No spawn: with nothing sent and a sender still held, the sleep wins.
#[tokio::test(start_paused = true)]
async fn expires_after_the_timeout_when_left_alone() {
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));
    let (_reset_tx, reset_rx) = mpsc::channel::<Signal>(16);
    let (expire_tx, expire_rx) = oneshot::channel();

    let now = Instant::now();
    let returned = watchdog.event_loop(reset_rx, expire_tx).await;

    assert_elapsed!(now, Duration::from_secs(1));
    assert_eq!(returned, watchdog);
    assert_eq!(expire_rx.await, Ok(Expired));
}

/// Restart means: make the channels again, run the loop again.
#[tokio::test(start_paused = true)]
async fn restarts_with_fresh_channels() {
    let watchdog = Watchdog::with_timeout(Duration::from_secs(1));

    let (_reset_tx, reset_rx) = mpsc::channel::<Signal>(16);
    let (expire_tx, expire_rx) = oneshot::channel();
    let watchdog = watchdog.event_loop(reset_rx, expire_tx).await;
    assert_eq!(expire_rx.await, Ok(Expired));

    let (_reset_tx, reset_rx) = mpsc::channel::<Signal>(16);
    let (expire_tx, expire_rx) = oneshot::channel();
    watchdog.event_loop(reset_rx, expire_tx).await;
    assert_eq!(expire_rx.await, Ok(Expired));
}
