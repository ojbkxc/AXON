// adapted from aisix/crates/aisix-ratelimit/src/store/mod.rs (Apache-2.0)

//! Pluggable counter backend behind the [`crate::Limiter`].

use axon_core::RateLimitConfig;
use async_trait::async_trait;

use crate::error::RateLimitError;
use crate::limiter::RateLimitStatus;

pub mod local;

pub(crate) const SECOND_SECS: u64 = 1;
pub(crate) const MINUTE_SECS: u64 = 60;
pub(crate) const HOUR_SECS: u64 = 60 * 60;
pub(crate) const DAY_SECS: u64 = 24 * 60 * 60;

pub(crate) struct Dim {
    pub name: &'static str,
    pub window_secs: u64,
    pub limit: u64,
}

pub(crate) fn request_dims(limits: &RateLimitConfig) -> Vec<Dim> {
    [
        ("rps", SECOND_SECS, limits.rps),
        ("rpm", MINUTE_SECS, limits.rpm),
        ("rph", HOUR_SECS, limits.rph),
        ("rpd", DAY_SECS, limits.rpd),
    ]
    .into_iter()
    .filter_map(|(name, window_secs, limit)| {
        limit.map(|limit| Dim {
            name,
            window_secs,
            limit,
        })
    })
    .collect()
}

pub(crate) fn token_dims(limits: &RateLimitConfig) -> Vec<Dim> {
    [
        ("tpm", MINUTE_SECS, limits.tpm),
        ("tpd", DAY_SECS, limits.tpd),
    ]
    .into_iter()
    .filter_map(|(name, window_secs, limit)| {
        limit.map(|limit| Dim {
            name,
            window_secs,
            limit,
        })
    })
    .collect()
}

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
