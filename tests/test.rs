#[cfg(test)]
mod tests {

    use directories::ProjectDirs;
    use number_prefix::NumberPrefix;
    use number_prefix::NumberPrefix::{Prefixed, Standalone};
    use rayon::prelude::*;
    use std::time::{Instant, SystemTime};
    use xorshift::{
        Rand, Rng, RngJump, SeedableRng, SplitMix64, Xoroshiro128, Xorshift1024, Xorshift128,
    };
    use rust_fast_cache::tools::{logger, fmt_bytes, get_nano_time};
    use rust_fast_cache::cache_service::cache::{Cache, ONE_MEBIBYTE};
    use rust_fast_cache::memdb::memory_database::{FastDB, DatabaseItem};

    #[test]
    fn test_cache() {
        logger::log("Starting Cache Test");
        let nowx = Instant::now();
        let mut cache_service: Cache = Cache::default();

        cache_service.insert_cache_item(String::from("TEST"), vec![0, 1, 2]);
        cache_service.insert_cache_item(String::from("TEST"), (0..255).map(u8::from).collect());

        cache_service.remove_cache_item("TEST");

        logger::log("Generating randomness");

        let mut sm: SplitMix64 = SeedableRng::from_seed(0);
        let mut rng: Xorshift1024 = Rand::rand(&mut sm);

        let mut test_data_n: Vec<usize> = (0u8..25).map(usize::from).collect();

        let max = ONE_MEBIBYTE * 10;
        let min = ONE_MEBIBYTE;

        let fxw: Vec<Vec<u8>> = test_data_n
            .par_iter()
            .map(|p| {
                let mut r = rng;
                r.jump(*p);

                let f = r.gen_range(min, max);
                let pref_num = fmt_bytes(f);

                logger::log(&format!("{}/{} [{}]", p, 25, pref_num));

                let mut numbers: Vec<u8> = vec![];
                for _ in 0..f {
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
        logger::debug(&format!("Len {:?}", t10.unwrap().len()));
        let mem_elapsed = now.elapsed();
        logger::debug(&format!("Elapsed {:?}", mem_elapsed));

        cache_service.resize_cache(Some(ONE_MEBIBYTE), None, None);

        let now = Instant::now();
        let t10 = cache_service
            .get_cache_value(String::from("TEST_1"))
            .expect("Err");
        logger::debug(&format!("Len {:?}", t10.unwrap().len()));
        let disk_elapsed = now.elapsed();
        logger::debug(&format!("Elapsed: {:?}", disk_elapsed));
        logger::debug(&format!(
            "Factor: {:?}",
            disk_elapsed.as_nanos() as f64 / mem_elapsed.as_nanos() as f64
        ));

        cache_service.resize_cache(Some(ONE_MEBIBYTE), Some(ONE_MEBIBYTE * 50), None);
        logger::log(&format!("{:?}", cache_service));

        logger::error(&format!("Finished cache testing in {:?}", nowx.elapsed()));
    }

    #[test]
    fn test_memdb() {
        let mut memdb = FastDB::default();
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
        let mut memdb = FastDB::default();
        let now = SystemTime::now();
        let max_i_1024: u64 = 1024;
        for i in 0..max_i_1024 {
            memdb.set(
                format!("{}", i + max_i_1024),
                DatabaseItem {
                    value: Some((0..255).map(u8::from).collect()),
                    last_access: get_nano_time(),
                    access_counter: 0,
                    filepath: None,
                },
            );
        }
        let elapsed_1024 = now.elapsed().unwrap();

        let mut memdb = FastDB::default();
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

        let mut memdb = FastDB::default();
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

        logger::log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_1024,
            elapsed_1024,
            elapsed_1024.as_nanos() as f64 / max_i_1024 as f64
        ));
        logger::log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_4096,
            elapsed_4096,
            elapsed_4096.as_nanos() as f64 / max_i_4096 as f64
        ));
        logger::log(&format!(
            "{:?}:\t{:?} ({:?} ns/insert)",
            max_i_16384,
            elapsed_16384,
            elapsed_16384.as_nanos() as f64 / max_i_16384 as f64
        ));
    }
}
