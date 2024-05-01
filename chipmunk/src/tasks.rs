use std::time::Duration;

use crate::database::tables::settings::Settings;
use crate::database::tables::token::Token;
use crate::database::tables::Tables;
use crate::database::DBTable;
use crate::logger::process_vehicle_data;
use crate::{database, EnvVars};
use anyhow::Context;
use backend::server::{MpscTopic, TeslaServer};
use tesla_api::stream::StreamingData;
use tesla_api::vehicle_data::VehicleData;
use tesla_api::{TeslaClient, TeslaError};
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio::sync::{broadcast, oneshot, watch};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Debug, Clone)]
enum DataTypes {
    VehicleData(String),
    StreamingData(StreamingData),
}

enum DatabaseDataType {
    RawData(String),
    Tables(Tables),
}

#[derive(Debug, Clone)]
enum Config<'a> {
    AccessToken(&'a str),
    LoggingPeriodMs(i32),
    Key(&'a str),
}

impl Default for Config<'_> {
    fn default() -> Self {
        Config::Key("default_key")
    }
}

async fn data_processor_task(
    mut vehicle_data_rx: mpsc::Receiver<DataTypes>,
    processed_data_tx: broadcast::Sender<Tables>,
    database_tx: mpsc::Sender<DatabaseDataType>,
    _config_rx: watch::Receiver<Config<'_>>,
    cancellation_token: CancellationToken,
    pool: &sqlx::PgPool,
) {
    use mpsc::error::*;
    let name = "data_processor_task";
    let mut vin_id_map = database::tables::car::get_vin_id_map(pool).await;
    let mut tables = Tables::db_get_last(pool).await;

    loop {
        tokio::task::yield_now().await;

        match vehicle_data_rx.try_recv() {
            Ok(v) => match v {
                DataTypes::VehicleData(data) => {
                    if let Err(e) = database_tx
                        .send(DatabaseDataType::RawData(data.clone()))
                        .await
                    {
                        log::error!("{name}: cannot send data over database_tx: {e}");
                    }

                    let vehicle_data = match VehicleData::from_response_json(&data) {
                        Ok(data) => data,
                        Err(e) => {
                            log::error!("Error parsing vehicle data to json: {e}");
                            continue;
                        }
                    };

                    (vin_id_map, tables) =
                        process_vehicle_data(pool, vin_id_map, tables, vehicle_data).await;

                    if let Err(e) = processed_data_tx.send(Tables::default()) {
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
    mut config_rx: watch::Receiver<Config<'_>>,
    cancellation_token: CancellationToken,
    vehicle_id: u64,
) {
    use mpsc::error::*;
    let name = "data_stream_task";
    let (streaming_data_tx, mut streaming_data_rx) = tokio::sync::mpsc::channel::<StreamingData>(1);
    let mut access_token = "";
    // TODO: Use the default access token if the config_rx is not available instead of
    // throwing error and breaking out of the loop
    if let Ok(true) = config_rx.has_changed() {
        access_token = match *config_rx.borrow_and_update() {
            Config::AccessToken(at) => at,
            _ => "",
        }
    }

    let access_token = access_token.to_string();
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
    config_rx: watch::Receiver<Config<'_>>,
    cancellation_token: CancellationToken,
    tesla_client: TeslaClient,
    car_id: u64,
) {
    let name = "data_polling_task";
    let mut _num_data_points = 0;
    loop {
        if cancellation_token.is_cancelled() {
            break;
        }

        // TODO: Use the default logging period if the config_rx is not available instead of
        // throwing error and breaking out of the loop
        let Config::LoggingPeriodMs(logging_period_ms) = *config_rx.borrow() else {
            log::error!("Error: cannot get logging period from config_rx");
            break;
        };

        match tesla_api::get_vehicle_data(&tesla_client, car_id).await {
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
    _config_rx: watch::Receiver<Config<'_>>,
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
                DatabaseDataType::Tables(_) => todo!(),
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
    _config_rx: watch::Receiver<Config<'_>>,
    cancellation_token: CancellationToken,
    http_port: u16,
) {
    use broadcast::error::*;
    let name = "web_server_task";

    let (data_from_server_tx, mut data_from_server_rx) = unbounded_channel();
    let (server_exit_signal_tx, server_exit_signal_rx) = oneshot::channel();
    let (data_to_server_tx, data_to_server_rx) = broadcast::channel::<i32>(1); // TODO: remove this channel and directly use data_rx to send data to web server

    let message_handler_task = tokio::task::spawn(async move {
        let name = format!("{name}::message_handler_task");
        loop {
            match data_from_server_rx.try_recv() {
                Ok(value) => match value {
                    MpscTopic::Logging(_value) => {
                        // TODO: Send log start command to logger task
                        // if let Err(e) = logger_tx.send(value) {
                        //     log::error!("{e}");
                        // }
                    }
                    MpscTopic::RefreshToken(refresh_token) => {
                        let _tokens =
                            match tesla_api::auth::refresh_access_token(refresh_token.as_str())
                                .await
                            {
                                Ok(t) => t,
                                Err(e) => {
                                    log::error!("{e}");
                                    continue;
                                }
                            };

                        // TODO: Send token to database task
                        // let encryption_key = env.encryption_key.clone();
                        // if let Err(e) = Token::db_insert(&pool, tokens, encryption_key.as_str()).await
                        // {
                        //     log::error!("{e}");
                        // }
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

            match data_rx.try_recv() {
                Ok(_data) => {
                    if let Err(e) = data_to_server_tx.send(1234) {
                        // TODO: Send data to web server
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
    });

    tokio::select! {
        result = TeslaServer::start(http_port, data_from_server_tx, data_to_server_rx, server_exit_signal_rx) => {
            match result {
                Ok(_) => log::warn!("web server exited"),
                Err(e) => log::error!("Web server exited: {e}"),
            }
        }
        status = message_handler_task => log::warn!("message handler task exited: {status:?}"),
    }
    tracing::warn!("exiting {name}");
}

pub async fn run(env: &EnvVars, pool: &sqlx::PgPool) -> anyhow::Result<()> {
    // channel to pass around system settings
    let (config_tx, config_rx) = watch::channel::<Config>(Config::default());
    // Channel for vehicle data and streaming data
    let (vehicle_data_tx, vehicle_data_rx) = mpsc::channel::<DataTypes>(1);
    // channel for parsed data
    let (processed_data_tx, data_rx) = broadcast::channel::<Tables>(1);
    // channel to send date to database task
    let (database_tx, database_rx) = mpsc::channel::<DatabaseDataType>(1);

    let cancellation_token = CancellationToken::new();
    let task_tracker = TaskTracker::new();

    let tokens = Token::db_get_last(pool, &env.encryption_key).await?;
    let tesla_client = tesla_api::get_tesla_client(&tokens.access_token)?;

    let vehicles = tesla_api::get_vehicles(&tesla_client).await?;
    let vehicle = vehicles.first(); // TODO: Use the first vehicle for now
    let car_id = vehicle
        .context("Invalid vehicle data")?
        .id
        .context("Invalid ID")?;

    let vehicle_id = vehicle
        .context("Invalid vehicle data")?
        .vehicle_id
        .context("Invalid vehicle ID")?;

    let settings = Settings::db_get_last(pool).await?;

    if let Err(e) = config_tx.send(Config::Key("new configuration key")) {
        log::error!("Error sending configuration: {e}");
    }
    if let Err(e) = config_tx.send(Config::LoggingPeriodMs(settings.logging_period_ms)) {
        log::error!("Error sending configuration: {e}");
    }

    // Transmits streaming data
    let data_stream_task_handle = {
        let vehicle_data_tx = vehicle_data_tx.clone();
        let config_rx = config_tx.subscribe();
        let cancellation_token = cancellation_token.clone();
        task_tracker.spawn(async move {
            data_streaming_task(vehicle_data_tx, config_rx, cancellation_token, vehicle_id).await;
        })
    };

    // Transmits polling data
    let data_polling_task_handle = {
        let vehicle_data_tx = vehicle_data_tx.clone();
        let cancellation_token = cancellation_token.clone();
        task_tracker.spawn(async move {
            data_polling_task(
                vehicle_data_tx,
                config_rx,
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
        let config_rx = config_tx.subscribe();
        let data_tx = processed_data_tx.clone();
        let database_tx = database_tx.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            data_processor_task(
                vehicle_data_rx,
                data_tx,
                database_tx,
                config_rx,
                cancellation_token,
                &pool,
            )
            .await;
        })
    };

    let database_task_handle = {
        let config_rx = config_tx.subscribe();
        let cancellation_token = cancellation_token.clone();
        let pool = pool.clone();
        task_tracker.spawn(async move {
            database_task(database_rx, config_rx, cancellation_token, &pool).await;
        })
    };

    // Starts web server and use the processed data to show logging status to the user
    let web_server_task_handle = {
        let config_rx = config_tx.subscribe();
        let cancellation_token = cancellation_token.clone();
        let http_port = env.http_port;
        task_tracker.spawn(async move {
            web_server_task(data_rx, config_rx, cancellation_token, http_port).await;
        })
    };

    // TODO: Task to periodically refresh access token

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
