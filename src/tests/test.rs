#[cfg(test)]
mod tests {
    use crate::cache_service::cache::{Cache, CleanseStrategy, ONE_GIBIBYTE, ONE_MEBIBYTE, ONE_BYTE};
    use crate::memdb::memory_database::{DatabaseItem, MemoryDatabase};
    use crate::tools::{get_nano_time, log, log_debug};
    use directories::ProjectDirs;
    use serde::de::Unexpected::Str;
    use std::time::{SystemTime, Instant};
    use rand::Rng;

    #[test]
    fn test_cache() {
        let mut cache_service: Cache = Cache::default();
        //println!("{:?}", cache_service);

        cache_service.insert_cache_item(String::from("TEST"), vec![0, 1, 2]);
        //println!("{:?}", cache_service);
        cache_service.insert_cache_item(String::from("TEST"), (0..255).map(u8::from).collect());
        //println!("{:?}", cache_service.get_cache_item(String::from("TEST")));
        //println!("{:?}", cache_service);
        cache_service.remove_cache_item("TEST");
        //println!("{:?}", cache_service);
        //println!("{:?}", cache_service.get_cache_item(String::from("TEST")));

        let mut rng = rand::thread_rng();
        for i in 0..25 {
            let mut numbers: Vec<u8> = vec![];
            for _ in 0..rng.gen_range(ONE_BYTE, ONE_MEBIBYTE) {
                numbers.push(rng.gen_range(0, 255));
            }

            cache_service.insert_cache_item(String::from(format!("TEST_{}", i)), numbers);
        }


        //Cache lookup test
        let now = Instant::now();
        let t10 = cache_service.get_cache_value(String::from("TEST_1")).expect("Err");
        log_debug(&format!("Len {:?}", t10.unwrap().len()));
        log_debug(&format!("Nanos {:?}", now.elapsed()));

        //println!("{:?}", cache_service);
        cache_service.resize_cache(Some(0), None, None);
        //println!("{:?}", cache_service);

        let now = Instant::now();
        let t10 = cache_service.get_cache_value(String::from("TEST_1")).expect("Err");
        log_debug(&format!("Len {:?}", t10.unwrap().len()));
        log_debug(&format!("Nanos {:?}", now.elapsed()));


    }

    #[test]
    fn test_memdb() {
        let mut memdb = MemoryDatabase::default();
        memdb.set(
            String::from("test"),
            DatabaseItem {
                value: Some(vec![0, 1]),
                last_access: get_nano_time(),
                access_counter: 0,
                filepath: None,
            },
        );

        let xvec = memdb.get("test").unwrap().unwrap();
        let xvec_val = xvec.value.unwrap();
        assert_eq!(xvec_val.len(), 2);
        assert_eq!(xvec_val, vec![0, 1]);

        memdb.del("test").unwrap().unwrap();

        assert!(memdb.get("test").unwrap().is_none())
    }

    #[test]
    fn test_mem_speed() {
        let mut memdb = MemoryDatabase::default();
        let now = SystemTime::now();
        let max_i_1024: u64 = 1024;
        for i in 0..max_i_1024 {
            memdb.set(
                format!("{:?}", i + max_i_1024),
                DatabaseItem {
                    value: Some((0..255).map(u8::from).collect()),
                    last_access: get_nano_time(),
                    access_counter: 0,
                    filepath: None,
                },
            );
        }
        let elapsed_1024 = now.elapsed().unwrap();

        let mut memdb = MemoryDatabase::default();
        let now = SystemTime::now();
        let max_i_4096: u64 = 4096;
        for i in 0..max_i_4096 {
            memdb.set(
                format!("{:?}", i + max_i_4096),
                DatabaseItem {
                    value: Some((0..255).map(u8::from).collect()),
                    last_access: get_nano_time(),
                    access_counter: 0,
                    filepath: None,
                },
            );
        }
        let elapsed_4096 = now.elapsed().unwrap();

        let mut memdb = MemoryDatabase::default();
        let now = SystemTime::now();
        let max_i_16384: u64 = 16384;
        for i in 0..max_i_16384 {
            memdb.set(
                format!("{:?}", i + max_i_16384),
                DatabaseItem {
                    value: Some((0..255).map(u8::from).collect()),
                    last_access: get_nano_time(),
                    access_counter: 0,
                    filepath: None,
                },
            );
        }
        let elapsed_16384 = now.elapsed().unwrap();

        println!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_1024,
            elapsed_1024,
            elapsed_1024.as_nanos() as f64 / max_i_1024 as f64
        );
        println!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_4096,
            elapsed_4096,
            elapsed_4096.as_nanos() as f64 / max_i_4096 as f64
        );
        println!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_16384,
            elapsed_16384,
            elapsed_16384.as_nanos() as f64 / max_i_16384 as f64
        );
    }
}
