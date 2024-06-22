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
    pub fn new(cache_dir: PathBuf) -> Self {
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).unwrap();
        }
        Self { cache_dir }
    }
}

impl Cache for FsCache {
    async fn set(&self, key: impl Hash, value: impl Serialize, expiry: Option<Duration>) {
        // Generates a unique hash for the key
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);

        // TODO: Figure out how to use a generic serializer instead of `serde_json`
        let value = serde_json::to_string(&value).unwrap();

        // Converts the hash to a string
        // and appends it to the cache directory
        // to create a unique file path
        let file_path = self.cache_dir.join(hash.finish().to_string());
        if file_path.exists() {
            fs::remove_file(&file_path).unwrap();
        }

        // Writes the value to the file
        fs::write(&file_path, value).unwrap();

        // Write the expiry time to a separate file
        if let Some(expiry) = expiry {
            let expiry_file_path = file_path.with_extension("expiry");
            let expires_at = Utc::now() + expiry;

            fs::write(&expiry_file_path, expires_at.timestamp().to_string()).unwrap();
        }
    }

    async fn get<'a, T>(&self, key: impl Hash) -> Option<T>
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
            let expiry = fs::read_to_string(&expiry_file_path)
                .unwrap()
                .parse::<i64>()
                .unwrap();
            let expiry = DateTime::from_timestamp(expiry, 0).unwrap();

            if expiry < Utc::now() {
                return None;
            }
        }

        // Converts the hash to a string
        // and appends it to the cache directory
        // to create a unique file path
        let file_path = self.cache_dir.join(hash.finish().to_string());

        match fs::read_to_string(&file_path) {
            Ok(value) => serde_json::from_str::<T>(&value).ok(),
            Err(_) => None,
        }
    }

    async fn invalidate(&self, key: impl Hash) {
        // Generates a unique hash for the key
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);

        let expiry_file_path = self
            .cache_dir
            .join(hash.finish().to_string())
            .with_extension("expiry");

        fs::write(&expiry_file_path, 0.to_string()).unwrap();
    }

    async fn collect_garbage(&self) {
        for file in fs::read_dir(&self.cache_dir).unwrap() {
            let file = file.unwrap();
            let path = file.path();
            if path.extension().is_some() {
                let extension = path.extension().unwrap().to_str().unwrap();
                if extension == "expiry" {
                    let expiry = fs::read_to_string(&path).unwrap().parse::<i64>().unwrap();
                    let expiry = DateTime::from_timestamp(expiry, 0).unwrap();
                    if expiry < Utc::now() {
                        let file_path = path.with_extension("");
                        fs::remove_file(file_path).unwrap();
                        fs::remove_file(path).unwrap();
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_flow() {
    let key = "Key";
    let cache = FsCache::new(PathBuf::from("data"));

    cache.set(key, "SOME_CUSTOM_VALUE", None).await;
    let value: String = cache.get(key).await.unwrap();
    assert_eq!("SOME_CUSTOM_VALUE", value);

    cache.invalidate(key).await;

    let new_value: Option<String> = cache.get(key).await;
    assert_eq!(None, new_value);

    cache.collect_garbage().await;
}
