#![feature(async_closure)]
#![feature(stmt_expr_attributes)]
#![feature(async_fn_in_trait)]

use std::io::Write;

pub mod crypto;
pub mod database;
pub mod environment;
pub mod location;
pub mod logger;
pub mod openstreetmap;
pub mod utils;

fn get_file_name(path_str: Option<&str>) -> String {
    if let Some(path_str_val) = path_str {
        let path = std::path::Path::new(path_str_val);
        if let Some(file_name) = path.file_name() {
            if let Some(s) = file_name.to_str() {
                return s.to_string();
            }
        }
    }

    "unknown".to_string()
}

pub fn init_log() {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            let level = record.level();
            let mut style = buf.style();
            match record.level() {
                log::Level::Error => style.set_color(env_logger::fmt::Color::Red),
                log::Level::Warn => style.set_color(env_logger::fmt::Color::Yellow),
                log::Level::Info => style.set_color(env_logger::fmt::Color::Green),
                log::Level::Debug => style.set_color(env_logger::fmt::Color::Blue),
                log::Level::Trace => style.set_color(env_logger::fmt::Color::Cyan),
            };
            let mut target_style = buf.style();
            target_style.set_color(env_logger::fmt::Color::Rgb(140, 143, 145));
            writeln!(
                buf,
                "{} [{}] {}{} {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                style.value(level),
                record.target(),
                target_style.value("]"),
                get_file_name(record.file()),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}
