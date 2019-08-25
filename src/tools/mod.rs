extern crate chrono;
use chrono::prelude::DateTime;
use chrono::Utc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use colored::*;
use crate::tools::Severity::{DEBUG, LOG, WARN, ERROR};

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


pub enum Severity{
    DEBUG,
    LOG,
    WARN,
    ERROR
}

pub fn log(log_obj: &str, severity: &Severity){
    match severity {
        Severity::DEBUG => {
            println!("{} {}", "[-]".green(), log_obj)
        },
        Severity::LOG => {
            println!("{} {}", "[+]".white(), log_obj)
        },
        Severity::WARN => {
            println!("{} {}", "[*]".yellow().bold(), log_obj)
        },
        Severity::ERROR => {
            println!("{} {}", "[!]".red().bold(), log_obj)
        },
    }
}

pub fn log_debug(log_obj: &str){
    log(log_obj, &DEBUG);
}
pub fn log_log(log_obj: &str){
    log(log_obj, &LOG);
}
pub fn log_warn(log_obj: &str){
    log(log_obj, &WARN);
}
pub fn log_error(log_obj: &str){
    log(log_obj, &ERROR);
}