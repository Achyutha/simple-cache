use serde::{de::DeserializeOwned, Serialize};
use std::{hash::Hash, time::Duration};

#[trait_variant::make()]
pub trait Cache {
    type Error;
    async fn set(
        &self,
        key: impl Hash,
        value: impl Serialize,
        expiry: Option<Duration>,
    ) -> Result<(), Self::Error>;
    async fn get<'a, T>(&self, key: impl Hash) -> Result<Option<T>, Self::Error>
    where
        T: DeserializeOwned;
    async fn invalidate(&self, key: impl Hash) -> Result<(), Self::Error>;
    async fn collect_garbage(&self) -> Result<(), Self::Error>;
}
