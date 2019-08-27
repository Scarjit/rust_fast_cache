use crate::cache_service::cache::CleanseStrategy;
use crate::tools;
use crate::tools::{fmt_bytes, get_nano_time, logger, nano_time_fmt, get_non_buffered_file_handle};
use parking_lot::{lock_api, RwLock};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::fs::{create_dir_all, remove_dir_all};
use std::hash::BuildHasherDefault;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};
use twox_hash::XxHash64;



#[derive(Clone)]
pub struct DatabaseItem {
    pub value: Option<Vec<u8>>,
    pub last_access: u128,
    pub access_counter: u64,
    pub filepath: Option<PathBuf>,
}
impl DatabaseItem {
    pub fn get_value_mem_size(&self) -> u64 {
        let val_len = match &self.value {
            Some(v) => v.len(),
            _ => 0,
        } as u64;
        let opt_vec = std::mem::size_of_val::<Option<Vec<u8>>>(&self.value) as u64;
        (std::mem::size_of::<u8>() as u64 * val_len) + opt_vec
    }

    pub fn get_mem_size(&self) -> u64 {
        self.get_value_mem_size()
            + std::mem::size_of::<u64>() as u64
            + std::mem::size_of::<u128>() as u64
            + std::mem::size_of_val::<Option<PathBuf>>(&self.filepath) as u64
    }
    pub fn get_disk_size(&self) -> io::Result<u64> {
        match &self.filepath {
            Some(v) => {
                if v.exists() {
                    Ok(fs::metadata(v)?.len())
                } else {
                    Ok(0)
                }
            }
            None => Ok(0),
        }
    }

    fn get_display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "filepath: {}, last_access: {}, access_counter: {}, value: {}, value_mem_size {}, mem_size: {}, disk_size: {}",
               format!("{:?}", self.filepath),
               nano_time_fmt(self.last_access),
               self.access_counter,
               format!("{:?}", self.value),
               self.get_value_mem_size(),
               self.get_mem_size(),
               format!("{:?}", self.get_disk_size())
        )
    }
}

impl Default for DatabaseItem {
    fn default() -> Self {
        Self {
            value: None,
            last_access: get_nano_time(),
            access_counter: 0,
            filepath: None,
        }
    }
}

impl fmt::Display for DatabaseItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.get_display(f)
    }
}

impl fmt::Debug for DatabaseItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.get_display(f)
    }
}

#[derive(Debug, Clone)]
pub struct FastDB {
    hashmap: Arc<RwLock<HashMap<String, DatabaseItem, BuildHasherDefault<XxHash64>>>>,
}

impl Default for FastDB {
    fn default() -> Self {
        Self {
            hashmap: Arc::new(RwLock::new(HashMap::<
                String,
                DatabaseItem,
                BuildHasherDefault<XxHash64>,
            >::default())),
        }
    }
}

impl FastDB {
    pub fn set(&mut self, key: String, value: DatabaseItem) -> io::Result<Option<DatabaseItem>> {
        let hashmap = Arc::<
            lock_api::RwLock<
                parking_lot::RawRwLock,
                HashMap<std::string::String, DatabaseItem, BuildHasherDefault<XxHash64>>,
            >,
        >::clone(&self.hashmap);
        let mut hashmap = hashmap.write();
        Ok(hashmap.insert(key, value))
    }

    pub fn get(&mut self, key: &str) -> io::Result<Option<DatabaseItem>> {
        let hashmap = &self.hashmap.read();
        let f = hashmap.get(key).cloned();
        Ok(f)
    }

    pub fn del(&mut self, key: &str) -> io::Result<Option<DatabaseItem>> {
        let hashmap = &mut self.hashmap.write();

        Ok(hashmap.remove(key))
    }

    pub fn cleanup_disk(
        &mut self,
        cleanup_strategy: &CleanseStrategy,
        mut to_clean: u64,
        cache_path: &str,
    ) -> io::Result<()> {
        let hashmap = Arc::<
            lock_api::RwLock<
                parking_lot::RawRwLock,
                HashMap<std::string::String, DatabaseItem, BuildHasherDefault<XxHash64>>,
            >,
        >::clone(&self.hashmap);

        let mut hashmap = hashmap.write();

        let keys = self.get_keys(&hashmap, cleanup_strategy);

        logger::warn(&format!("{} {} {:?}", to_clean, cache_path, keys));

        let mut to_remove: Vec<String> = vec![];

        for k in keys {
            if to_clean == 0 {
                break;
            }

            match &k.4 {
                Ok(v) => {
                    to_remove.push(k.0.clone());

                    if to_clean >= *v {
                        to_clean -= *v;
                    } else {
                        to_clean = 0;
                    }
                }
                Err(v) => {
                    logger::error(&format!("\t\tSkipping {:?} ERROR: {:?}", k.0, v));
                }
            }
        }

        for k in &to_remove {
            let folder_path = format!("{}/{}", cache_path, k);

            if Path::new(&folder_path).exists() {
                remove_dir_all(&folder_path)?;
            }

            hashmap.remove(k);
        }

        logger::debug(&format!("\tKeys to remove ({:?}): {:?}", &to_remove.len() , &to_remove));

        Ok(())
    }

    fn get_keys(
        &mut self,
        hashmap: &lock_api::RwLockWriteGuard<
            '_,
            parking_lot::RawRwLock,
            std::collections::HashMap<
                std::string::String,
                DatabaseItem,
                std::hash::BuildHasherDefault<twox_hash::XxHash64>,
            >,
        >,
        cleanup_strategy: &CleanseStrategy,
    ) -> Vec<(String, u64, u128, u64, io::Result<u64>)> {
        let mut keys: Vec<(String, u64, u128, u64, io::Result<u64>)> = vec![];

        for (k, v) in hashmap.iter() {
            keys.push((
                k.to_owned(),
                v.access_counter,
                v.last_access,
                v.get_mem_size(),
                v.get_disk_size(),
            ))
        }

        match cleanup_strategy {
            CleanseStrategy::LastAccess => {
                keys.sort_by(|a, b| a.2.cmp(&b.2));
            }
            CleanseStrategy::LeastUsed => {
                keys.sort_by(|a, b| a.1.cmp(&b.1));
            }
            CleanseStrategy::Combined => {
                keys.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
            }
        }
        keys
    }

    pub fn cleanup_mem(
        &mut self,
        cleanup_strategy: &CleanseStrategy,
        mut to_clean: u64,
        cache_path: &str,
    ) -> io::Result<u64> {
        let hashmap = Arc::<
            lock_api::RwLock<
                parking_lot::RawRwLock,
                HashMap<std::string::String, DatabaseItem, BuildHasherDefault<XxHash64>>,
            >,
        >::clone(&self.hashmap);
        let mut hashmap = hashmap.write();

        let keys = self.get_keys(&hashmap, cleanup_strategy);

        let mut to_disk: Vec<String> = vec![];

        for k in keys {
            if to_clean == 0 {
                break;
            }
            logger::log("");
            logger::log(&format!("\t\tLeft to clean:{}", fmt_bytes(to_clean)));
            logger::log(&format!(
                "\t\t{:?}: {:?} {:?} {:?} {:?}",
                &k.0,
                &k.1,
                tools::nano_time_fmt(k.2),
                &k.3,
                &k.4
            ));

            match &k.4 {
                Ok(v) => {
                    if v == &0 {
                        to_disk.push(k.0.clone());
                        logger::debug(&format!(
                            "\t\tMoving {:?} to disk will yield: {}",
                            &k.0,
                            fmt_bytes(k.3)
                        ));
                        if k.3 <= to_clean {
                            to_clean -= k.3;
                        } else {
                            to_clean = 0;
                        }
                    }
                }
                Err(v) => {
                    logger::error(&format!("\t\tSkipping {:?} ERROR: {:?}", k.0, v));
                }
            }
        }

        logger::debug(&format!("\tKeys to disk ({:?}): {:?}", &to_disk.len() , &to_disk));

        let mut ds: u64 = 0;

        for k in to_disk {
            let mut f = hashmap.get(&k).cloned().expect("Key went missing");

            let folder_path = format!("{}/{}", cache_path, k);
            f.filepath = Some(PathBuf::from(&folder_path));

            if Path::new(&folder_path).exists() {
                remove_dir_all(&folder_path)?;
            }

            create_dir_all(&folder_path)?;

            let file_path = format!("{}/cachefile", &folder_path);

            let mut file = get_non_buffered_file_handle(&file_path)?;

            let value = &f.value.expect("f has no value !");
            file.write_all(value)?;

            f.filepath = Some(PathBuf::from(&file_path));
            f.value = None;

            ds += f.get_disk_size()?;

            hashmap.insert(k, f);
        }

        Ok(ds)
    }
}
