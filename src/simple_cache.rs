use serde::{de::DeserializeOwned, Serialize};
use std::{hash::Hash, time::Duration};

#[trait_variant::make()]
pub trait Cache {
    async fn set(&self, key: impl Hash, value: impl Serialize, expiry: Option<Duration>);
    async fn get<'a, T>(&self, key: impl Hash) -> Option<T>
    where
        T: DeserializeOwned;
    async fn invalidate(&self, key: impl Hash);
    async fn collect_garbage(&self);
}
