#![feature(async_closure)]
#![feature(stmt_expr_attributes)]

use std::{env, io::Write, ops::Deref};

use anyhow::{anyhow, Context};
use config::Config;
use database::tables::vehicle_data;
use tesla_api::vehicle_data::VehicleData;
use tokio::sync::mpsc;

pub mod charging;
pub mod config;
pub mod database;
pub mod logger;
pub mod openstreetmap;
pub mod server;
pub mod tasks;
pub mod utils;

pub const DELAYED_DATAPOINT_TIME_SEC: i64 = 10 * 60;

#[derive(Clone)]
pub struct EnvVars {
    pub encryption_key: String,
    pub database_url: String,
    pub car_data_database_url: Option<String>,
    pub http_port: u16,
}

// TODO: move this function to config.rs
pub fn load_env_vars() -> anyhow::Result<EnvVars> {
    let encryption_key =
        env::var("TOKEN_ENCRYPTION_KEY").context("Please provide TOKEN_ENCRYPTION_KEY")?;
    let database_url = env::var("DATABASE_URL").context("Please provide DATABASE_URL")?;
    let car_data_database_url = match env::var("CAR_DATA_DATABASE_URL") {
        Ok(v) => Some(v),
        Err(e) => match e {
            env::VarError::NotPresent => None,
            env::VarError::NotUnicode(e) => {
                anyhow::bail!(
                    "Invalid value for environment variable CAR_DATA_DATABASE_URL: {e:?}"
                );
            }
        },
    };

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
        car_data_database_url,
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
            let gray = env_logger::fmt::style::RgbColor::from((140, 143, 145)).on_default().render();

            writeln!(
                buf,
                "{timestamp} [{style}{level}{style_reset}] {crate_name}{gray}]{style_reset} {filename}:{line_num} - {message}"
            )
        })
        .init();
}

pub async fn convert_db(
    env: &EnvVars,
    pool: &sqlx::PgPool,
    config: &Config,
    num_rows_to_fetch: i64,
) -> anyhow::Result<()> {
    let Some(ref car_data_database_url) = env.car_data_database_url else {
        anyhow::bail!("Please provide CAR_DATA_DATABASE_URL");
    };
    let car_data_pool = database::initialize_car_data(car_data_database_url).await?;

    // Channel to send vehicle data
    let (vehicle_data_tx, vehicle_data_rx) = mpsc::channel::<tasks::DataTypes>(1);

    let num_rows = vehicle_data::num_car_data_rows(&car_data_pool).await?;
    let batch_size = if num_rows_to_fetch < 10_000 {
        num_rows_to_fetch
    } else {
        10_000
    };
    let mut row_offset = num_rows - num_rows_to_fetch;

    let tasks = crate::tasks::convert_db(pool, config, vehicle_data_rx);

    let fetch_data_task = tokio::task::spawn({
        async move {
            while (row_offset - batch_size) < num_rows {
                let data_list = vehicle_data::db_get(&car_data_pool, batch_size, row_offset)
                    .await
                    .map_err(|e| anyhow!(e))?;

                for data in data_list {
                    let data_str = match serde_json::to_string::<VehicleData>(data.deref()) {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("Error converting vehicle data to string: {e}");
                            anyhow::bail!("Error converting vehicle data to string: {e}");
                        }
                    };

                    if let Err(e) = vehicle_data_tx
                        .send(tasks::DataTypes::VehicleData(data_str))
                        .await
                    {
                        log::error!("{e}");
                        anyhow::bail!(e);
                    }
                }
                row_offset += batch_size;
            }
            Ok(())
        }
    });

    tokio::select! {
        status = tasks => log::warn!("task handler exited: {status:?}"),
        status = fetch_data_task => log::warn!("fetch data task exited: {status:?}"),
    }
    tracing::warn!("exiting convertdb");
    Ok(())
}
