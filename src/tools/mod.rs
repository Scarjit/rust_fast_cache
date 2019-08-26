extern crate chrono;
use chrono::prelude::DateTime;
use chrono::Utc;
use number_prefix::NumberPrefix;
use number_prefix::NumberPrefix::{Prefixed, Standalone};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn get_nano_time() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos()
}

pub fn nano_time_fmt(nano: u128) -> String {
    let d = UNIX_EPOCH + Duration::from_nanos(nano as u64);
    let datetime = DateTime::<Utc>::from(d);
    // Formats the combined date and time with the specified format string.
    datetime.format("%Y-%m-%d %H:%M:%S.%f").to_string()
}

pub fn fmt_bytes(b: u64) -> String {
    match NumberPrefix::binary(b as f64) {
        Standalone(bytes) => format!("{} bytes", bytes),
        Prefixed(prefix, n) => format!("{:.0} {}B", n, prefix),
    }
}

pub mod logger {
    use colored::Colorize;

    pub enum Severity {
        DEBUG,
        LOG,
        WARN,
        ERROR,
    }

    fn l(log_obj: &str, severity: &Severity) {
        match severity {
            Severity::DEBUG => println!("{} {}", "[-]".green(), log_obj),
            Severity::LOG => println!("{} {}", "[+]".white(), log_obj),
            Severity::WARN => println!("{} {}", "[*]".yellow().bold(), log_obj),
            Severity::ERROR => println!("{} {}", "[!]".red().bold(), log_obj),
        }
    }

    pub fn debug(log_obj: &str) {
        l(log_obj, &Severity::DEBUG);
    }
    pub fn log(log_obj: &str) {
        l(log_obj, &Severity::LOG);
    }
    pub fn warn(log_obj: &str) {
        l(log_obj, &Severity::WARN);
    }
    pub fn error(log_obj: &str) {
        l(log_obj, &Severity::ERROR);
    }
}
