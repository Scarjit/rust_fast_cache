use crate::cache_service::cache::CleanseStrategy;
use crate::tools;
use crate::tools::{fmt_bytes, get_nano_time, log_debug, log_log, log_warn, nano_time_fmt};
use parking_lot::{lock_api, RwLock};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::fs::OpenOptions;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::hash::BuildHasherDefault;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
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
        let size_of_optvec8 = self.get_value_mem_size();
        let size_of_u64 = std::mem::size_of::<u64>() as u64;
        let size_of_systemtime = std::mem::size_of::<u128>() as u64;
        let size_of_optpathbuf = std::mem::size_of_val::<Option<PathBuf>>(&self.filepath) as u64;
        let f1: u64 = size_of_optvec8
            .checked_add(size_of_u64)
            .expect("Couldn't get memory size");
        let f2: u64 = size_of_systemtime
            .checked_add(size_of_optpathbuf)
            .expect("Couldn't get memory size");

        f1.checked_add(f2).expect("Couldn't get memory size")
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
pub struct MemoryDatabase {
    hashmap: Arc<RwLock<HashMap<String, DatabaseItem, BuildHasherDefault<XxHash64>>>>,
    locked_r: u64,
    locked_w: u64,
}

impl Default for MemoryDatabase {
    fn default() -> Self {
        Self {
            hashmap: Arc::new(RwLock::new(HashMap::<
                String,
                DatabaseItem,
                BuildHasherDefault<XxHash64>,
            >::default())),
            locked_r: 0,
            locked_w: 0,
        }
    }
}

impl MemoryDatabase {
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

    pub fn cleanup(
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

        let mut keys: Vec<(String, u64, u128, u64, io::Result<u64>)> = vec![];

        for (k, v) in hashmap.iter() {
            //println!("{:?}: {:?}", k, v);
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

        let mut to_disk: Vec<String> = vec![];

        for k in keys {
            if to_clean == 0 {
                break;
            }
            log_log("");
            log_log(&format!("\t\tLeft to clean:{}", fmt_bytes(to_clean)));
            log_log(&format!(
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
                        log_debug(&format!(
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
                    log_warn(&format!("\t\tSkipping {:?} ERROR: {:?}", k.0, v));
                }
            }
        }

        log_debug(&format!("\tKeys to disk: {:?}", to_disk));

        for k in to_disk {
            let mut f = hashmap.get(&k).cloned().expect("Key went missing");

            let folder_path = format!("{}/{}", cache_path, k);
            f.filepath = Some(PathBuf::from(&folder_path));

            if Path::new(&folder_path).exists() {
                remove_dir_all(&folder_path)?;
            }

            create_dir_all(&folder_path)?;

            let file_path = format!("{}/cachefile", &folder_path);

            //Disable file caching for linux
            let mut file: File = if cfg!(target_os = "linux") {
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .custom_flags(libc::O_DIRECT)
                    .open(&file_path)
                    .expect("Could not open file")
            } else {
                File::create(&file_path)?
            };

            let value = &f.value.expect("f has no value !");
            file.write_all(value)?;

            f.filepath = Some(PathBuf::from(&file_path));
            f.value = None;

            hashmap.insert(k, f);
        }

        Ok(())
    }
}
