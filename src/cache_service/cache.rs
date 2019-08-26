use crate::memdb::memory_database::{DatabaseItem, MemoryDatabase};
use crate::tools::get_nano_time;
use directories::ProjectDirs;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::io;
use std::path::Path;
extern crate rand;
use crate::tools::logger;
use rand::Rng;
use std::fs::File;
use std::io::Read;

pub const ONE_BYTE: u64 = 1;
pub const ONE_KIBIBYTE: u64 = ONE_BYTE * 1024;
pub const ONE_MEBIBYTE: u64 = ONE_KIBIBYTE * 1024;
pub const ONE_GIBIBYTE: u64 = ONE_MEBIBYTE * 1024;
pub const TEN_GIBIBYTE: u64 = ONE_GIBIBYTE * 10;

pub const ONE_SECOND: u64 = 1;
pub const ONE_MINUTE: u64 = ONE_SECOND * 60;
pub const ONE_HOUR: u64 = ONE_MINUTE * 60;
pub const ONE_DAY: u64 = ONE_HOUR * 24;

/// Defines multiple strategies for cleaning up the cache.
/// * `LastAccess` : Sorts files by access time and removes oldest
/// * `LeastUsed` : Removes least used files.
/// * `Combined` : Sorts by usage and then removes files by age.
#[derive(Debug)]
pub enum CleanseStrategy {
    LastAccess,
    LeastUsed,
    Combined,
}

/// Cache manager
/// * `max_ram_cache` : Amount of ram in bytes to use for caching. [Default: 1GiB]
/// * `max_disk_cache` : Amount of disk in bytes to use for caching. [Default: 10 GiB]
/// * `decache_age` : Amount of seconds after which a file is auto de-cached. [Default: 1 Day]
/// * `cache_path` : Path to on disk cache [Default: Depends on OS]
#[derive(Debug)]
pub struct Cache {
    max_ram_cache: u64,
    max_disk_cache: u64,
    decache_age: u64,
    cache_path: String,
    memdb: MemoryDatabase,
    memdb_size: u64,
    diskdb_size: u64,
    management_threadpool: ThreadPool,
}

impl Default for Cache {
    fn default() -> Self {
        let pd = ProjectDirs::from("net", "soontm", "rust_fast_cache")
            .expect("Default cache dir not found!");

        Self {
            max_ram_cache: ONE_GIBIBYTE,
            max_disk_cache: TEN_GIBIBYTE,
            decache_age: ONE_DAY,
            cache_path: String::from(
                pd.cache_dir()
                    .to_str()
                    .expect("Couldn't get default cache path"),
            ),
            memdb: MemoryDatabase::default(),
            memdb_size: 0,
            diskdb_size: 0,
            management_threadpool: ThreadPoolBuilder::new()
                .num_threads(num_cpus::get_physical())
                .build()
                .expect("Couldn't create threadpool"),
        }
    }
}

impl Cache {
    /// Set or change the cache path
    /// WARNING: Old path will not be cleared !
    pub fn set_cache_path(&mut self, new_cache_path: String) {
        if !Path::new(&new_cache_path).exists() {
            logger::log("Cache path does not exist, creating!");
            std::fs::create_dir_all(&new_cache_path).expect("Could not create cache path");
        }
        self.cache_path = new_cache_path;
    }

    /// Change cache settings.
    /// * `max_ram_cache` : Amount of ram in bytes to use for caching. [Default: 1GiB]
    /// * `max_disk_cache` : Amount of disk in bytes to use for caching. [Default: 10 GiB]
    /// * `cleanse_strategy` : How to remove cache data, if full. [Default CleanseStrategy::Combined]
    pub fn resize_cache(
        &mut self,
        max_ram_cache: Option<u64>,
        max_disk_cache: Option<u64>,
        cleanse_strategy: Option<CleanseStrategy>,
    ) {
        logger::warn("Resizing cache, no requests will be handled !");
        let new_max_ram = max_ram_cache.unwrap_or(ONE_GIBIBYTE);
        let new_max_disk = max_disk_cache.unwrap_or(TEN_GIBIBYTE);
        let c_strat = cleanse_strategy.unwrap_or(CleanseStrategy::Combined);

        if new_max_ram < self.max_ram_cache {
            self.cleanup_mem_cache(&c_strat, new_max_ram)
                .expect("Couldn't cleanup memory");
        }

        if new_max_disk < self.max_disk_cache {
            self.cleanup_disk_cache(&c_strat, new_max_disk);
        }

        self.max_ram_cache = new_max_ram;
        self.max_disk_cache = new_max_disk;
        logger::warn("Resized cache, requests will be handled again !");
    }

    fn cleanup_mem_cache(
        &mut self,
        cleanse_strategy: &CleanseStrategy,
        new_max_cache: u64,
    ) -> io::Result<()> {
        if self.memdb_size <= new_max_cache {
            return Ok(());
        }

        let to_clean = self
            .memdb_size
            .checked_sub(new_max_cache)
            .expect("New max_cache < memdb size");

        logger::log("[CLEANING MEMDB]");
        logger::log(&format!("\tMemory used: {:?}", &self.memdb_size));
        logger::log(&format!("\tMemory max: {:?}", new_max_cache));
        logger::log(&format!("\tCleaning up: {:?}", to_clean));
        logger::log(&format!("\tStartegy: {:?}", cleanse_strategy));

        self.memdb
            .cleanup(cleanse_strategy, to_clean, &self.cache_path.to_owned())?;

        Ok(())
    }

    fn cleanup_disk_cache(&mut self, _cleanse_strategy: &CleanseStrategy, _new_max_disk: u64) {}

    pub fn remove_cache_item(&mut self, key: &str) -> io::Result<Option<DatabaseItem>> {
        let dbi = self.memdb.get(key)?;
        match dbi {
            Some(v) => {
                let size = v.get_mem_size();
                self.memdb.del(key)?;
                self.memdb_size -= size;

                if v.filepath.is_some() {
                    std::fs::remove_dir_all(v.filepath.expect("Filepath not existent :("))?;
                }

                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub fn insert_cache_item(
        &mut self,
        key: String,
        value: Vec<u8>,
    ) -> io::Result<Option<DatabaseItem>> {
        let mut rng = rand::thread_rng();

        self.remove_cache_item(&key.clone())?;

        //let file_path:PathBuf = PathBuf::from(format!("{}/{}",&self.cache_path ,&key));
        let dbi = DatabaseItem {
            value: Some(value),
            last_access: get_nano_time(),
            access_counter: rng.gen_range(0, 3), //TODO remove after testing !
            filepath: None,
        };
        self.memdb_size += dbi.get_mem_size();
        Ok(self.memdb.set(key, dbi)?)
    }

    pub fn get_cache_item(&mut self, key: String) -> io::Result<Option<DatabaseItem>> {
        let f = self.memdb.get(&key)?;
        if f.is_none() {
            return Ok(None);
        }

        let mut fx = f.expect("Some is None !");
        fx.last_access = get_nano_time();
        fx.access_counter += 1;

        self.memdb.set(key, fx.clone())?;

        Ok(Some(fx))
    }

    pub fn get_cache_value(&mut self, key: String) -> io::Result<Option<Vec<u8>>> {
        let cache_item = self.get_cache_item(key)?;
        if cache_item.is_none() {
            return Ok(None);
        }

        let fxi = cache_item.expect("Some is none");
        match fxi.value {
            None => match fxi.filepath {
                None => Ok(None),
                Some(v) => {
                    if Path::new(&v).exists() {
                        let mut f = File::open(&v)?;
                        let mut buff: Vec<u8> = vec![];
                        f.read_to_end(&mut buff)?;
                        logger::log("From disk");
                        Ok(Some(buff))
                    } else {
                        Ok(None)
                    }
                }
            },
            Some(v) => {
                logger::log("From memory");
                Ok(Some(v))
            }
        }
    }
}
