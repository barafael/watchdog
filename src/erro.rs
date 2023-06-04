use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// The watchdog is no longer active due to having dropped.
    #[error("Watchdog is no longer active due to having dropped")]
    Inactive,
}
