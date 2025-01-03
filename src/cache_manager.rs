use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::workspace::{get_cache_path, get_out_dir};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CacheRecord {
    url: String,
    name: String,
}

pub struct CacheManager;
impl CacheManager {
    pub fn initialize() {
        if !get_cache_path().exists() {
            fs::write(get_cache_path(), "{}").unwrap();
        }
    }

    fn cache_records() -> HashMap<String, CacheRecord> {
        let cache_records: HashMap<String, CacheRecord> =
            serde_json::from_str(fs::read_to_string(get_cache_path()).unwrap().as_str()).unwrap();

        cache_records
    }

    pub fn contains(url: &Url) -> bool {
        Self::get_record(url).is_some()
    }

    fn get_record(url: &Url) -> Option<CacheRecord> {
        match Self::cache_records().get(url.as_str()) {
            Some(record) => Some(record.to_owned()),
            None => None,
        }
    }

    pub fn get_video_path(url: &Url) -> std::path::PathBuf {
        get_out_dir().join(Self::get_record(url).unwrap().name)
    }

    pub fn add_record(url: Url, file_name: String) {
        let mut cache_records = Self::cache_records();
        cache_records.insert(
            url.to_string(),
            CacheRecord {
                url: url.to_string(),
                name: file_name,
            },
        );
        fs::write(
            get_cache_path(),
            serde_json::to_string_pretty(&cache_records).unwrap(),
        )
        .unwrap();
        dbg!(fs::read_to_string(get_cache_path()).unwrap());
    }
}
