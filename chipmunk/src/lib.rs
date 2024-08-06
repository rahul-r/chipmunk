#![feature(async_closure)]
#![feature(stmt_expr_attributes)]
#![allow(
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unused_async,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::explicit_deref_methods,
    clippy::needless_pass_by_value,
    clippy::large_futures,
    clippy::manual_let_else,
    clippy::manual_string_new,
    clippy::too_many_lines,
    clippy::ignored_unit_patterns,
    clippy::semicolon_if_nothing_returned,
    clippy::useless_attribute,
    clippy::wildcard_imports,
    clippy::no_effect_underscore_binding,
    clippy::needless_raw_string_hashes,
    clippy::match_same_arms
)]

use std::io::Write;

pub mod charging;
pub mod config;
pub mod database;
pub mod logger;
pub mod openstreetmap;
pub mod server;
pub mod srtm;
mod task_data_polling;
pub mod task_data_processor;
pub mod task_data_streaming;
mod task_database;
mod task_web_server;
pub mod tasks;
pub mod utils;

pub const DELAYED_DATAPOINT_TIME_SEC: i64 = 10 * 60;

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
            let level_style = buf.default_level_style(level);
            let style = level_style.render();
            let style_reset = level_style.render_reset();
            let timestamp = buf.timestamp();
            let filename = get_file_name(record.file());
            let line_num = record.line().unwrap_or(0);
            let message = record.args();
            let crate_name = record.target();
            let gray = env_logger::fmt::style::RgbColor::from((140, 143, 145)).on_default().render();

            writeln!(
                buf,
                "{timestamp} [{style}{level}{style_reset}] {crate_name}{gray}]{style_reset} {filename}:{line_num} - {message}"
            )
        })
        .init();
}
