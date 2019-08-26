#[cfg(test)]
mod tests {
    use crate::cache_service::cache::{
        Cache, CleanseStrategy, ONE_BYTE, ONE_GIBIBYTE, ONE_KIBIBYTE, ONE_MEBIBYTE,
    };
    use crate::memdb::memory_database::{DatabaseItem, MemoryDatabase};
    use crate::tools::{fmt_bytes, get_nano_time, log, log_debug, log_error, log_log};
    use directories::ProjectDirs;
    use number_prefix::NumberPrefix;
    use number_prefix::NumberPrefix::{Prefixed, Standalone};
    use rayon::prelude::*;
    use std::time::{Instant, SystemTime};
    use xorshift::{
        Rand, Rng, RngJump, SeedableRng, SplitMix64, Xoroshiro128, Xorshift1024, Xorshift128,
    };

    #[test]
    fn test_cache() {
        log_log("Starting Cache Test");
        let nowx = Instant::now();
        let mut cache_service: Cache = Cache::default();

        cache_service.insert_cache_item(String::from("TEST"), vec![0, 1, 2]);
        cache_service.insert_cache_item(String::from("TEST"), (0..255).map(u8::from).collect());

        cache_service.remove_cache_item("TEST");

        log_log("Generating randomness");

        let mut sm: SplitMix64 = SeedableRng::from_seed(0);
        let mut rng: Xorshift1024 = Rand::rand(&mut sm);

        let mut test_data_n: Vec<usize> = (0u8..25).map(usize::from).collect();

        let fxw: Vec<Vec<u8>> = test_data_n
            .par_iter()
            .map(|p| {
                let mut r = rng;
                r.jump(*p);
                //let max = ONE_MEBIBYTE * 10;
                let max = ONE_KIBIBYTE * 10;

                let pref_num = fmt_bytes(max);

                log_log(&format!("{}/{} [{}]", p, 25, pref_num));

                let mut numbers: Vec<u8> = vec![];
                for _ in 0..max {
                    numbers.push(r.gen_range(0, 255));
                }
                numbers
            })
            .collect();

        for i in 0..25 {
            cache_service.insert_cache_item(
                String::from(format!("TEST_{}", i)),
                fxw.get(i).unwrap().to_owned(),
            );
        }
        //Cache lookup test
        let now = Instant::now();
        let t10 = cache_service
            .get_cache_value(String::from("TEST_1"))
            .expect("Err");
        log_debug(&format!("Len {:?}", t10.unwrap().len()));
        let mem_elapsed = now.elapsed();
        log_debug(&format!("Elapsed {:?}", mem_elapsed));

        cache_service.resize_cache(Some(0), None, None);

        let now = Instant::now();
        let t10 = cache_service
            .get_cache_value(String::from("TEST_1"))
            .expect("Err");
        log_debug(&format!("Len {:?}", t10.unwrap().len()));
        let disk_elapsed = now.elapsed();
        log_debug(&format!("Elapsed: {:?}", disk_elapsed));
        log_debug(&format!(
            "Factor: {:?}",
            disk_elapsed.as_nanos() as f64 / mem_elapsed.as_nanos() as f64
        ));

        log_error(&format!("Finished cache testing in {:?}", nowx.elapsed()));
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
                format!("{:", i + max_i_1024),
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
                format!("{}", i + max_i_4096),
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
                format!("{}", i + max_i_16384),
                DatabaseItem {
                    value: Some((0..255).map(u8::from).collect()),
                    last_access: get_nano_time(),
                    access_counter: 0,
                    filepath: None,
                },
            );
        }
        let elapsed_16384 = now.elapsed().unwrap();

        log_log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_1024,
            elapsed_1024,
            elapsed_1024.as_nanos() as f64 / max_i_1024 as f64
        ));
        log_log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_4096,
            elapsed_4096,
            elapsed_4096.as_nanos() as f64 / max_i_4096 as f64
        ));
        log_log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_16384,
            elapsed_16384,
            elapsed_16384.as_nanos() as f64 / max_i_16384 as f64
        ));
    }
}
