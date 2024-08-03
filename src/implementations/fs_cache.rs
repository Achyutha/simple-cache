use chrono::{DateTime, Utc};
use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    time::Duration,
};

use serde::{de::DeserializeOwned, Serialize};

use crate::simple_cache::Cache;

pub struct FsCache {
    cache_dir: PathBuf,
}

impl FsCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self, std::io::Error> {
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
        }
        Ok(Self { cache_dir })
    }
}

impl Cache for FsCache {
    type Error = std::io::Error;
    async fn set(
        &self,
        key: impl Hash,
        value: impl Serialize,
        expiry: Option<Duration>,
    ) -> Result<(), Self::Error> {
        // Generates a unique hash for the key
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);

        // TODO: Figure out how to use a generic serializer instead of `serde_json`
        let value = serde_json::to_string(&value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Converts the hash to a string
        // and appends it to the cache directory
        // to create a unique file path
        let file_path = self.cache_dir.join(hash.finish().to_string());
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }

        // Writes the value to the file
        fs::write(&file_path, value)?;

        // Write the expiry time to a separate file
        if let Some(expiry) = expiry {
            let expiry_file_path = file_path.with_extension("expiry");
            let expires_at = Utc::now() + expiry;

            fs::write(&expiry_file_path, expires_at.timestamp().to_string())?;
        }

        Ok(())
    }

    async fn get<'a, T>(&self, key: impl Hash) -> Result<Option<T>, Self::Error>
    where
        T: DeserializeOwned,
    {
        // Generates a unique hash for the key
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);

        // Check if the expiry file exists. If it does. Check if the expiry time has passed
        let expiry_file_path = self
            .cache_dir
            .join(hash.finish().to_string())
            .with_extension("expiry");

        if expiry_file_path.exists() {
            let expiry = fs::read_to_string(&expiry_file_path)?
                .parse::<i64>()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let expiry = DateTime::from_timestamp(expiry, 0);

            if let Some(expiry) = expiry {
                if expiry < Utc::now() {
                    return Ok(None);
                }
            }
        }

        // Converts the hash to a string
        // and appends it to the cache directory
        // to create a unique file path
        let file_path = self.cache_dir.join(hash.finish().to_string());

        let value = fs::read_to_string(&file_path)?;
        let res = serde_json::from_str::<T>(&value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Some(res))
    }

    async fn invalidate(&self, key: impl Hash) -> Result<(), Self::Error> {
        // Generates a unique hash for the key
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);

        let expiry_file_path = self
            .cache_dir
            .join(hash.finish().to_string())
            .with_extension("expiry");

        fs::write(&expiry_file_path, 0.to_string())?;
        Ok(())
    }

    async fn collect_garbage(&self) -> Result<(), Self::Error> {
        for file in fs::read_dir(&self.cache_dir)? {
            let file = file?;
            let path = file.path();
            if let Some(extension) = path.extension() {
                if let Some(extension) = extension.to_str() {
                    if extension == "expiry" {
                        let expiry = fs::read_to_string(&path)?
                            .parse::<i64>()
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                        if let Some(expiry) = DateTime::from_timestamp(expiry, 0) {
                            if expiry < Utc::now() {
                                let file_path = path.with_extension("");
                                fs::remove_file(file_path)?;
                                fs::remove_file(path)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
