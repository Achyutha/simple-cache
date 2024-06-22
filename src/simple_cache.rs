use serde::Serialize;
use std::{hash::Hash, time::Duration};

#[trait_variant::make()]
pub trait Cache {
    async fn set(&self, key: impl Hash, value: impl Serialize, expiry: Option<Duration>);
    async fn get(&self, key: impl Hash) -> Option<impl Serialize>;
    async fn invalidate(&self, key: impl Hash);
    async fn clear(&self);
}
