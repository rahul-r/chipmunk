#![feature(async_closure)]
#![feature(stmt_expr_attributes)]
#![feature(async_fn_in_trait)]
#![feature(result_option_inspect)]

use std::{env, io::Write};

use anyhow::Context;

pub mod charging;
pub mod car;
pub mod database;
pub mod logger;
pub mod openstreetmap;
pub mod utils;

pub struct EnvVars {
    pub encryption_key: String,
    pub database_url: String,
    pub http_port: u16,
}

pub fn load_env_vars() -> anyhow::Result<EnvVars> {
    let encryption_key =
        env::var("TOKEN_ENCRYPTION_KEY").context("Please provide TOKEN_ENCRYPTION_KEY")?;
    let database_url = env::var("DATABASE_URL").context("Please provide DATABASE_URL")?;

    const DEFAULT_PORT: u16 = 3072;
    let http_port = match env::var("HTTP_PORT") {
        Ok(port) => port.parse().unwrap_or_else(|e| {
            log::error!("Invalid HTTP_PORT `{port}`: {e}");
            log::info!("Using default port {DEFAULT_PORT}");
            DEFAULT_PORT
        }),
        Err(e) => {
            log::warn!("Error reading HTTP_PORT from environment: {e}");
            log::info!("Using default port {DEFAULT_PORT}");
            DEFAULT_PORT
        }
    };

    Ok(EnvVars {
        encryption_key,
        database_url,
        http_port,
    })
}

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
            let grey = env_logger::fmt::style::RgbColor::from((140, 143, 145)).on_default().render();

            writeln!(
                buf,
                "{timestamp} [{style}{level}{style_reset}] {crate_name}{grey}]{style_reset} {filename}:{line_num} - {message}"
            )
        })
        .init();
}
