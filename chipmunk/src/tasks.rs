use anyhow::anyhow;
use std::ops::Deref;
use std::time::Duration;

use crate::config::Config;
use crate::database::tables::token::Token;
use crate::database::tables::{vehicle_data, Tables};
use crate::task_data_polling::data_polling_task;
use crate::task_data_processor::data_processor_task;
use crate::task_data_streaming::data_streaming_task;
use crate::task_database::database_task;
use crate::task_web_server::web_server_task;
use crate::{database, get_config, set_config};
use tesla_api::stream::StreamingData;
use tesla_api::vehicle_data::VehicleData;
use tesla_api::{TeslaClient, TeslaError};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Debug, Clone)]
pub enum DataTypes {
    VehicleData(String),
    StreamingData(StreamingData),
}

pub enum DatabaseDataType {
    RawData(String),
    Tables(Vec<Tables>),
}

pub enum DatabaseRespType {
    _RawData(String),
    Tables(Tables),
}

fn handle_token_expiry(_pool: &sqlx::PgPool) {
    log::info!("Running `handle_token_expiry` callback");
}

pub async fn run(pool: &sqlx::PgPool, config: &mut Config) -> anyhow::Result<()> {
    // Channel for vehicle data and streaming data
    let (vehicle_data_tx, vehicle_data_rx) = mpsc::channel::<DataTypes>(1);
    // channel for parsed data
    let (processed_data_tx, data_rx) = broadcast::channel::<Tables>(1);
    // channel to send date to database task
    let (database_tx, database_rx) = mpsc::channel::<DatabaseDataType>(1);
    // channel to receive response from database task
    let (database_resp_tx, database_resp_rx) = mpsc::channel::<DatabaseRespType>(1);

    let cancellation_token = CancellationToken::new();
    let task_tracker = TaskTracker::new();

    // Starts web server and use the processed data to show logging status to the user
    let web_server_task_handle = {
        let config = config.clone();
        let pool = pool.clone();
        let cancellation_token = cancellation_token.clone();

        task_tracker.spawn(async move {
            let tables = Tables::db_get_last(&pool).await;
            web_server_task(data_rx, config, &tables, cancellation_token).await;
        })
    };

    let encryption_key = match config.encryption_key.lock().map(|c| c.get()) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            anyhow::bail!("{e}");
        }
    };
    // Read tokens from the database if exists, if not, get from the user and store in the database
    let tokens = match Token::db_get_last(pool, &encryption_key).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("{e}");
            log::info!("Waiting for auth token from user. Enter the token using the web interface");

            // Wait for the user to supply auth token via the web interface
            set_config!(config.refresh_token, "".into());
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let refresh_token = get_config!(config.refresh_token).unwrap_or_else(|e| {
                    log::error!("Error getting config value for `refresh_token`: {e}");
                    "".into()
                });
                if refresh_token.is_empty() {
                    continue;
                }

                match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
                    Ok(tokens) => {
                        Token::db_insert(pool, &tokens, encryption_key.as_str()).await?;
                        break;
                    }
                    Err(e) => {
                        log::error!("{e}");
                        continue;
                    }
                };
            }
            Token::db_get_last(pool, &encryption_key).await?
        }
    };

    let pool_clone = pool.clone();
    let mut tesla_client = tesla_api::get_tesla_client(
        tokens.clone(),
        Some(Box::new(move || handle_token_expiry(&pool_clone))),
    )?;

    let (car_id, vehicle_id) = match get_ids(&mut tesla_client).await {
        Some((car_id, vehicle_id)) => (car_id, vehicle_id),
        None => anyhow::bail!("Cannot read vehicle IDs"),
    };

    // Transmits streaming data
    let data_stream_task_handle = {
        let vehicle_data_tx = vehicle_data_tx.clone();
        let config = config.clone();
        let cancellation_token = cancellation_token.clone();
        task_tracker.spawn(async move {
            data_streaming_task(vehicle_data_tx, config, cancellation_token, vehicle_id).await;
        })
    };

    // Transmits polling data
    let data_polling_task_handle = {
        let vehicle_data_tx = vehicle_data_tx.clone();
        let config = config.clone();
        let cancellation_token = cancellation_token.clone();
        task_tracker.spawn(async move {
            data_polling_task(
                vehicle_data_tx,
                config,
                cancellation_token,
                tesla_client,
                car_id,
            )
            .await;
        })
    };

    // Receives polling and streaming data, parse the data and transmits the processed data
    let data_processor_task_handle = {
        let cancellation_token = cancellation_token.clone();
        let data_tx = processed_data_tx.clone();
        let config = config.clone();
        let database_tx = database_tx.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            data_processor_task(
                vehicle_data_rx,
                data_tx,
                database_tx,
                database_resp_rx,
                config,
                cancellation_token,
                &pool,
            )
            .await;
        })
    };

    let database_task_handle = {
        let cancellation_token = cancellation_token.clone();
        let config = config.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            database_task(
                database_rx,
                database_resp_tx,
                config,
                cancellation_token,
                &pool,
            )
            .await;
        })
    };

    // After spawning all the tasks, close the tracker
    task_tracker.close();

    // Wait for any one of the tasks to exit
    tokio::select! {
        status = data_processor_task_handle => tracing::info!("logger task done: {:?}", status),
        status = data_stream_task_handle => tracing::info!("data stream task done: {:?}", status),
        status = data_polling_task_handle => tracing::info!("data polling task done: {:?}", status),
        status = database_task_handle => tracing::info!("database task done: {:?}", status),
        status = web_server_task_handle => tracing::info!("web server task done: {:?}", status),
        _ = tokio::signal::ctrl_c() => tracing::info!("Ctrl+C received"),
    }

    tracing::info!("stopping tasks and exiting...");
    // One or more tasks exited, tell the remaining tasks to exit
    cancellation_token.cancel();
    // Wait for all tasks to exit
    task_tracker.wait().await;

    Ok(())
}

async fn get_ids(tesla_client: &mut TeslaClient) -> Option<(u64, u64)> {
    match tesla_api::get_vehicles(tesla_client).await {
        Ok(vehicles) => {
            let vehicle = vehicles.first(); // FIXME: Use the first vehicle for now
            let Some(car_id) = vehicle.and_then(|v| v.id) else {
                log::error!("Cannot read id field from vehicle_data");
                return None;
            };
            let Some(vehicle_id) = vehicle.and_then(|v| v.vehicle_id) else {
                log::error!("Cannot read vehicle_id field from vehicle_data");
                return None;
            };
            Some((car_id, vehicle_id))
        }
        Err(e) => {
            match e {
                TeslaError::TokenExpired(_) => (),
                TeslaError::Retry(e) => log::warn!("{e}"),
                e => log::error!("Error: {e}"),
            }
            None
        }
    }
}

pub async fn convert_db(
    pool: &sqlx::PgPool,
    config: &Config,
    num_rows_to_fetch: i64,
) -> anyhow::Result<()> {
    let car_data_database_url = config
        .car_data_database_url
        .lock()
        .map(|c| c.get())
        .map_err(|e| log::error!("Error reading `car_data_database_url` from config: {e}"))
        .ok()
        .flatten();

    let Some(ref car_data_database_url) = car_data_database_url else {
        anyhow::bail!("Please provide CAR_DATA_DATABASE_URL");
    };
    let car_data_pool = database::initialize_car_data(car_data_database_url).await?;

    // Channel for vehicle data and streaming data
    let (vehicle_data_tx, vehicle_data_rx) = mpsc::channel::<DataTypes>(1);
    // channel for parsed data
    let (processed_data_tx, _data_rx) = broadcast::channel::<Tables>(1);
    // channel to send date to database task
    let (database_tx, database_rx) = mpsc::channel::<DatabaseDataType>(1);
    // channel to receive response from database task
    let (database_resp_tx, database_resp_rx) = mpsc::channel::<DatabaseRespType>(1);

    let cancellation_token = CancellationToken::new();
    let task_tracker = TaskTracker::new();

    // Receives polling and streaming data, parse the data and transmits the processed data
    let data_processor_task_handle = {
        let cancellation_token = cancellation_token.clone();
        let data_tx = processed_data_tx.clone();
        let config = config.clone();
        let database_tx = database_tx.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            data_processor_task(
                vehicle_data_rx,
                data_tx,
                database_tx,
                database_resp_rx,
                config,
                cancellation_token,
                &pool,
            )
            .await;
        })
    };

    let database_task_handle = {
        let cancellation_token = cancellation_token.clone();
        let config = config.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            database_task(
                database_rx,
                database_resp_tx,
                config,
                cancellation_token,
                &pool,
            )
            .await;
        })
    };

    let num_rows = vehicle_data::num_car_data_rows(&car_data_pool).await?;
    let batch_size = if num_rows_to_fetch < 10_000 {
        num_rows_to_fetch
    } else {
        10_000
    };
    let mut row_offset = num_rows - num_rows_to_fetch;

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

                    if let Err(e) = vehicle_data_tx.send(DataTypes::VehicleData(data_str)).await {
                        log::error!("{e}");
                        anyhow::bail!(e);
                    }
                }
                row_offset += batch_size;
            }
            Ok(())
        }
    });

    // After spawning all the tasks, close the tracker
    task_tracker.close();

    // Wait for any one of the tasks to exit
    tokio::select! {
        status = data_processor_task_handle => tracing::info!("logger task done: {:?}", status),
        status = database_task_handle => tracing::info!("database task done: {:?}", status),
        status = fetch_data_task => log::warn!("fetch data task exited: {status:?}"),
        _ = tokio::signal::ctrl_c() => tracing::info!("Ctrl+C received"),
    }

    tracing::info!("stopping tasks and exiting convertdb...");
    // One or more tasks exited, tell the remaining tasks to exit
    cancellation_token.cancel();
    // Wait for all tasks to exit
    task_tracker.wait().await;

    Ok(())
}
