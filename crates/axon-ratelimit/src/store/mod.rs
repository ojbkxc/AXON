// adapted from aisix/crates/aisix-ratelimit/src/store/mod.rs (Apache-2.0)

//! Pluggable counter backend behind the [`crate::Limiter`].

use async_trait::async_trait;
use axon_core::RateLimitConfig;

use crate::error::RateLimitError;
use crate::limiter::RateLimitStatus;

pub mod local;

pub(crate) const SECOND_SECS: u64 = 1;
pub(crate) const MINUTE_SECS: u64 = 60;
pub(crate) const HOUR_SECS: u64 = 60 * 60;
pub(crate) const DAY_SECS: u64 = 24 * 60 * 60;

#[async_trait]
pub trait RateStore: Send + Sync + 'static {
    async fn acquire(
        &self,
        key: &str,
        limits: &RateLimitConfig,
        member: &str,
    ) -> Result<(), RateLimitError>;

    async fn commit(&self, key: &str, tokens: u64, member: &str);

    fn release(&self, key: &str, member: &str);

    fn add_tokens(&self, key: &str, tokens: u64);

    async fn peek(&self, key: &str, limits: &RateLimitConfig) -> Option<RateLimitStatus>;
}
