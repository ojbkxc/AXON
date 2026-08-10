// adapted from aisix/crates/aisix-ratelimit/src/lib.rs (Apache-2.0)

//! axon-ratelimit — two-phase RPM/TPM/concurrency limiter.
//!
//! The server middleware calls [`Limiter::pre_commit`] before dispatching
//! a chat request; the returned [`Reservation`] is finalised with
//! [`Reservation::commit_tokens`] after the upstream response completes.

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod clock;
mod error;
mod limiter;
pub mod store;
mod window;

pub use clock::{Clock, SystemClock, TestClock};
pub use error::RateLimitError;
pub use limiter::{
    Limiter, MultiReservation, RateLimitStatus, Reservation, StreamConcurrencyGuard,
};
pub use store::local::LocalStore;
pub use store::RateStore;
pub use window::{FixedWindowCounter, WindowCheck};
