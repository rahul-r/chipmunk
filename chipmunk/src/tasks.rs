use std::time::Duration;

use crate::config::{Config, ConfigItem as ci};
use crate::database::tables::token::Token;
use crate::database::tables::Tables;
use crate::logger::{create_tables, get_car_id};
use crate::server::{DataToServer, MpscTopic, TeslaServer};
use crate::{database, EnvVars};
use tesla_api::stream::StreamingData;
use tesla_api::vehicle_data::VehicleData;
use tesla_api::{TeslaClient, TeslaError};
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use ui_common::LoggingStatus;

#[derive(Debug, Clone)]
pub enum DataTypes {
    VehicleData(String),
    StreamingData(StreamingData),
}

enum DatabaseDataType {
    RawData(String),
    Tables(Vec<Tables>),
}

enum DatabaseRespType {
    _RawData(String),
    Tables(Tables),
}

async fn data_processor_task(
    mut vehicle_data_rx: mpsc::Receiver<DataTypes>,
    processed_data_tx: broadcast::Sender<Tables>,
    database_tx: mpsc::Sender<DatabaseDataType>,
    mut database_resp_rx: mpsc::Receiver<DatabaseRespType>,
    _config: Config,
    cancellation_token: CancellationToken,
    pool: &sqlx::PgPool,
) {
    use mpsc::error::*;
    let name = "data_processor_task";
    let mut vin_id_map = database::tables::car::get_vin_id_map(pool).await;
    let mut prev_tables = Tables::db_get_last(pool).await;

    loop {
        tokio::task::yield_now().await;

        match vehicle_data_rx.try_recv() {
            Ok(v) => match v {
                DataTypes::VehicleData(data) => {
                    if let Err(e) = database_tx
                        .send(DatabaseDataType::RawData(data.clone()))
                        .await
                    {
                        log::error!("{name}: cannot send raw vehicle data over database_tx: {e}");
                    }

                    let vehicle_data = match VehicleData::from_response_json(&data) {
                        Ok(data) => data,
                        Err(e) => {
                            log::error!("Error parsing vehicle data to json: {e}");
                            continue;
                        }
                    };

                    let car_id_opt;
                    (vin_id_map, car_id_opt) = get_car_id(pool, vin_id_map, &vehicle_data).await;

                    let Some(car_id) = car_id_opt else {
                        log::error!("Error getting car ID");
                        continue;
                    };

                    let table_list = match create_tables(&vehicle_data, &prev_tables, car_id).await
                    {
                        Ok(table_list) => table_list,
                        Err(e) => {
                            log::error!("Error adding to database: {e}");
                            continue;
                        }
                    };

                    // Send the tables to the database task
                    if let Err(e) = database_tx.send(DatabaseDataType::Tables(table_list)).await {
                        log::error!("{name}: cannot send table_list over database_tx: {e}");
                    }

                    // Wait for the response from database task with the updated tables with
                    // database id fields
                    if let Some(resp) = database_resp_rx.recv().await {
                        if let DatabaseRespType::Tables(prev_tables_resp) = resp {
                            prev_tables = prev_tables_resp;
                        } else {
                            log::error!("Unexpected response type received from database task");
                        }
                    } else {
                        log::error!("No response received from database task");
                    }

                    if let Err(e) = processed_data_tx.send(prev_tables.clone()) {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
                DataTypes::StreamingData(_data) => {
                    if let Err(e) = processed_data_tx.send(Tables::default()) {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
            },
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                // don't log error message if the channel was disconnected because of cancellation request
                if !cancellation_token.is_cancelled() {
                    log::error!("vehicle_data_rx channel disconnected, exiting {name}");
                }
                break;
            }
        }
        if cancellation_token.is_cancelled() {
            break;
        }
    }
    tracing::warn!("exiting {name}");
}

async fn data_streaming_task(
    data_tx: mpsc::Sender<DataTypes>,
    config: Config,
    cancellation_token: CancellationToken,
    vehicle_id: u64,
) {
    use mpsc::error::*;
    let name = "data_stream_task";
    let (streaming_data_tx, mut streaming_data_rx) = tokio::sync::mpsc::channel::<StreamingData>(1);

    let ci::AccessToken(access_token) = config.get(&ci::AccessToken("".into())) else {
        log::error!("Invalid access token");
        return;
    };

    let streaming_data_tx = streaming_data_tx.clone();
    let cancellation_token_clone = cancellation_token.clone();
    let streaming_task = tokio::task::spawn_blocking(async move || {
        tesla_api::stream::start(
            &access_token,
            vehicle_id,
            streaming_data_tx,
            cancellation_token_clone,
        )
        .await
        .map_err(|e| log::error!("Error streaming: {e}"))
        .ok();
        log::warn!("Vehicle data streaming stopped");
    });

    loop {
        match streaming_data_rx.try_recv() {
            Ok(data) => {
                if let Err(e) = data_tx.send(DataTypes::StreamingData(data)).await {
                    // don't log error message if the channel was closed because of a cancellation request
                    if !cancellation_token.is_cancelled() {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                // don't log error message if the channel was disconnected because of cancellation request
                if !cancellation_token.is_cancelled() {
                    log::error!("streaming_data_rx channel disconnected, exiting {name}");
                }
                break;
            }
        }
        if cancellation_token.is_cancelled() {
            break;
        }
        tokio::task::yield_now().await;
    }

    if let Err(e) = streaming_task.await {
        log::error!("Error waiting for streaming task: {e}");
    }

    tracing::warn!("exiting {name}");
}

async fn data_polling_task(
    data_tx: mpsc::Sender<DataTypes>,
    config: Config,
    cancellation_token: CancellationToken,
    mut tesla_client: TeslaClient,
    car_id: u64,
) {
    let name = "data_polling_task";
    let mut _num_data_points = 0;
    loop {
        if cancellation_token.is_cancelled() {
            break;
        }

        if !config.get(&ci::LoggingEnabled(false)).get_bool() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let logging_period_ms = config.get(&ci::LoggingPeriodMs(0)).get_i32();

        match tesla_api::get_vehicle_data(&mut tesla_client, car_id).await {
            Ok(data) => {
                if let Err(e) = data_tx.send(DataTypes::VehicleData(data)).await {
                    // don't log error message if the channel was closed because of a cancellation request
                    if !cancellation_token.is_cancelled() {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
            }
            Err(e) => {
                match e {
                    TeslaError::Connection(e) => log::error!("Error: `{e}`"),
                    TeslaError::Request(e) => log::error!("Error: `{e}`"),
                    TeslaError::ApiError(e) => log::error!("Error: `{e}`"), // TODO: Error: `429 - Account or server is rate limited. This happens when too many requests are made by an account.
                    // • Check the 'Retry-After' request header (in seconds); to determine when to make the next request.`
                    TeslaError::NotOnline => {
                        // TODO: Is there a way to wait for the vehicle to come online?
                        log::info!("Vehicle is not online");
                    }
                    TeslaError::InvalidHeader(e) => log::error!("Error: `{e}`"),
                    TeslaError::ParseError(e) => log::error!("Error: `{e}`"),
                    TeslaError::WebSocketError(e) => log::error!("Error: `{e}`"),
                    TeslaError::TokenExpired(e) => log::error!("Error: `{e}`"),
                    TeslaError::JsonDecodeError(e) => log::error!("Error: `{e}`"),
                    TeslaError::RequestTimeout => log::info!("Timeout"),
                    TeslaError::InvalidResponse(ref msg) => log::error!("Error: `{e}` - {msg}"),
                    TeslaError::TestInProgress => log::info!("{e}"),
                    TeslaError::Retry(e) => log::info!("{e}"),
                }
                tokio::time::sleep(Duration::from_millis(logging_period_ms as u64)).await;
                continue;
            }
        };

        _num_data_points += 1;

        tokio::time::sleep(Duration::from_millis(logging_period_ms as u64)).await;
    }

    tracing::warn!("exiting {name}");
}

async fn database_task(
    mut data_rx: mpsc::Receiver<DatabaseDataType>,
    data_resp_tx: mpsc::Sender<DatabaseRespType>,
    _config: Config,
    cancellation_token: CancellationToken,
    pool: &sqlx::PgPool,
) {
    use mpsc::error::*;
    let name = "database_task";
    loop {
        match data_rx.try_recv() {
            Ok(data) => match data {
                DatabaseDataType::RawData(d) => {
                    if let Err(e) = database::tables::vehicle_data::db_insert_json(&d, pool).await {
                        log::error!("{e}");
                    };
                }
                DatabaseDataType::Tables(table_list) => {
                    let mut last_tables = Tables::default();
                    for t in table_list {
                        match t.db_insert(pool).await {
                            Ok(updated_tables) => last_tables = updated_tables,
                            Err(e) => log::error!("Error inserting tables into database: {:?}", e),
                        }
                    }
                    if let Err(e) = data_resp_tx
                        .send(DatabaseRespType::Tables(last_tables))
                        .await
                    {
                        log::error!("Error sending response from database task: {e}");
                    }
                }
            },
            Err(TryRecvError::Disconnected) => {
                // don't log error message if the channel was closed because of a cancellation request
                if !cancellation_token.is_cancelled() {
                    log::error!("data_rx channel closed, exiting {name}");
                }
                break;
            }
            Err(TryRecvError::Empty) => (),
        }
        if cancellation_token.is_cancelled() {
            break;
        }
        tokio::task::yield_now().await;
    }
    tracing::warn!("exiting {name}");
}

async fn web_server_task(
    mut data_rx: broadcast::Receiver<Tables>,
    config: Config,
    cancellation_token: CancellationToken,
    http_port: u16,
) {
    use broadcast::error::*;
    let name = "web_server_task";

    let (data_from_server_tx, mut data_from_server_rx) = unbounded_channel();
    let (server_exit_signal_tx, server_exit_signal_rx) = oneshot::channel();
    let (data_to_server_tx, data_to_server_rx) = broadcast::channel::<DataToServer>(1);

    let message_handler_task = tokio::task::spawn({
        let mut config = config.clone();
        async move {
            let name = format!("{name}::message_handler_task");
            let mut logging_status = ui_common::LoggingStatus::default();
            loop {
                match data_from_server_rx.try_recv() {
                    Ok(value) => match value {
                        MpscTopic::Logging(value) => config.set(ci::LoggingEnabled(value)),
                        MpscTopic::RefreshToken(refresh_token) => {
                            if let Err(e) =
                                tesla_api::auth::refresh_access_token(refresh_token.as_str())
                                    .await
                                    .map(|t| {
                                        config.set(ci::AccessToken(t.access_token));
                                        config.set(ci::RefreshToken(t.refresh_token));
                                    })
                            {
                                log::error!("{e}");
                                continue;
                            }
                        }
                    },
                    Err(e) => match e {
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                            log::error!("server_rx channel closed, exiting {name}");
                            break;
                        }
                        tokio::sync::mpsc::error::TryRecvError::Empty => (),
                    },
                }

                let is_logging = config.get(&ci::LoggingEnabled(false)).get_bool();

                match data_rx.try_recv() {
                    Ok(data) => {
                        let odometer = data.position.unwrap().odometer.unwrap() as i32;
                        logging_status = LoggingStatus {
                            timestamp: chrono::offset::Utc::now(),
                            is_logging,
                            current_points: logging_status.current_points + 1,
                            total_points: logging_status.total_points + 1,
                            current_miles: odometer,
                            total_miles: odometer,
                            is_user_present: true,
                            odometer,
                            charging_status: "TBD".into(),
                            charge_state: "TBD".into(),
                        };
                        if let Err(e) = data_to_server_tx
                            .send(DataToServer::LoggingStatus(logging_status.clone()))
                        {
                            log::error!("Error sending data to web server: {e}");
                        }
                    }
                    Err(TryRecvError::Closed) => {
                        // don't log error message if the channel was closed because of a cancellation request
                        if !cancellation_token.is_cancelled() {
                            log::error!("data_rx channel closed, exiting {name}");
                        }
                        break;
                    }
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Lagged(n)) => {
                        log::warn!("{name} lagged too far behind; {n} messages skipped")
                    }
                }
                if cancellation_token.is_cancelled() {
                    if let Err(e) = server_exit_signal_tx.send(()) {
                        log::error!("Error sending exit signal to server: {e:?}")
                    }
                    break;
                }
                tokio::task::yield_now().await;
            }
        }
    });

    tokio::select! {
        result = TeslaServer::start(config, http_port, data_from_server_tx, data_to_server_rx, server_exit_signal_rx) => {
            match result {
                Ok(_) => log::warn!("web server exited"),
                Err(e) => log::error!("Web server exited: {e}"),
            }
        }
        status = message_handler_task => log::warn!("message handler task exited: {status:?}"),
    }
    tracing::warn!("exiting {name}");
}

fn handle_token_expiry(_pool: &sqlx::PgPool) {
    log::info!("Running `handle_token_expiry` callback");
}

pub async fn run(env: &EnvVars, pool: &sqlx::PgPool, config: &mut Config) -> anyhow::Result<()> {
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
        let cancellation_token = cancellation_token.clone();
        let http_port = env.http_port;
        task_tracker.spawn(async move {
            web_server_task(data_rx, config, cancellation_token, http_port).await;
        })
    };

    // Read tokens from the database if exists, if not, get from the user and store in the databse
    let tokens = match Token::db_get_last(pool, &env.encryption_key).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("{e}");
            log::info!("Waiting for auth token from user. Enter the token using the web interface");

            // Wait for the user to supply auth token via the web interface
            config.set(ci::RefreshToken("".into()));
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let refresh_token = config.get(&ci::RefreshToken("".into())).get_string();
                if refresh_token.is_empty() {
                    continue;
                }

                match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
                    Ok(tokens) => {
                        Token::db_insert(pool, &tokens, env.encryption_key.as_str()).await?;
                        break;
                    }
                    Err(e) => {
                        log::error!("{e}");
                        continue;
                    }
                };
            }
            Token::db_get_last(pool, &env.encryption_key).await?
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
    vehicle_data_rx: mpsc::Receiver<DataTypes>,
) -> anyhow::Result<()> {
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

    // After spawning all the tasks, close the tracker
    task_tracker.close();

    // Wait for any one of the tasks to exit
    tokio::select! {
        status = data_processor_task_handle => tracing::info!("logger task done: {:?}", status),
        status = database_task_handle => tracing::info!("database task done: {:?}", status),
        _ = tokio::signal::ctrl_c() => tracing::info!("Ctrl+C received"),
    }

    tracing::info!("stopping tasks and exiting...");
    // One or more tasks exited, tell the remaining tasks to exit
    cancellation_token.cancel();
    // Wait for all tasks to exit
    task_tracker.wait().await;

    Ok(())
}
