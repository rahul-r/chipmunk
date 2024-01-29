use std::sync::mpsc;

use anyhow::Context;
use backend::server::{MpscTopic, TeslaServer};
use tesla_api::{
    auth::AuthResponse,
    get_tesla_client, get_vehicle_data, get_vehicles,
    stream::{self, StreamingData},
    vehicle_data::VehicleData,
    TeslaClient, TeslaError,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::{sleep, Duration};

use crate::{
    database::{
        self,
        tables::{settings::Settings, DBTable, Tables},
    },
    environment::Environment,
};

pub async fn log(pool: &sqlx::PgPool, env: &Environment) -> anyhow::Result<()> {
    let (server_tx, mut server_rx) = tokio::sync::mpsc::unbounded_channel();
    let ui_server = TeslaServer::start(env.http_port, server_tx);

    let (logger_tx, logger_rx) = tokio::sync::mpsc::unbounded_channel();

    // Make copies so that we can move these into the future without causing borrow errors
    let encryption_key = env.encryption_key.clone();
    let pool1 = pool.clone();
    let pool2 = pool.clone();

    let cmd_handler = tokio::task::spawn(async move {
        while let Some(topic) = server_rx.recv().await {
            match topic {
                MpscTopic::Logging(value) => {
                    if let Err(e) = logger_tx.send(value) {
                        log::error!("{e}");
                    }
                }
                MpscTopic::RefreshToken(refresh_token) => {
                    let tokens =
                        match tesla_api::auth::refresh_access_token(refresh_token.as_str()).await {
                            Ok(t) => t,
                            Err(e) => {
                                log::error!("{e}");
                                continue;
                            }
                        };
                    if let Err(e) =
                        database::token::insert(&pool1, tokens, encryption_key.as_str()).await
                    {
                        log::error!("{e}");
                    }
                }
            }
        }
    });

    let status_reporter = tokio::task::spawn(async move {
        loop {
            let srv = ui_server.lock().await;
            let msg = srv.get_status_str();
            srv.broadcast(msg).await;
            sleep(Duration::from_secs(1)).await;
        }
    });

    tokio::select! {
        res = cmd_handler => res?,
        res = status_reporter => res?,
        res = start(&pool2, &env.encryption_key, logger_rx) => res?,
    }

    Ok(())
}

async fn start(
    pool: &sqlx::PgPool,
    encryption_key: &str,
    mut rx: UnboundedReceiver<bool>,
) -> anyhow::Result<()> {
    let mut message_shown = false;
    loop {
        if !database::token::exists(pool).await? {
            if !message_shown {
                log::info!(
                    "Cannot find Tesla auth tokens in database, waiting for token from user"
                );
                message_shown = true;
            }
            sleep(Duration::from_secs(2)).await;
            continue;
        };

        let tokens = database::token::get(pool, encryption_key).await?;

        let tesla_client = get_tesla_client(&tokens.access_token)?;

        if let Err(e) = logging_process(pool, &tesla_client, &tokens, &mut rx).await {
            log::error!("Error logging vehicle data: {e}, restarting the logger...");
        } else {
            log::error!("Logging stopped");
            break;
        }

        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

async fn get_vehicle_data_task(
    mut start_logger_signal_rx: UnboundedReceiver<bool>,
    client_clone: TeslaClient,
    car_id: u64,
    vehicle_data_tx: UnboundedSender<String>,
    settings: Settings,
) {
    let mut logging_status = false;
    let mut _num_data_points = 0;
    loop {
        use tokio::sync::mpsc::error::*;
        match start_logger_signal_rx.try_recv() {
            Ok(v) => {
                logging_status = v;
                // TODO: Send logging_status to ui server
                // ui_server.lock().await.set_logging_status(logging_status);
            }
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                log::error!("Logger disconnected");
                break;
            }
        }

        if !logging_status {
            // Logging is disabled, wait for logging to be enabled
            match start_logger_signal_rx.recv().await {
                Some(v) => {
                    logging_status = v;
                    // TODO: Send logging_status to ui server
                    // ui_server.lock().await.set_logging_status(logging_status);
                }
                None => {
                    log::error!("Logger disconnected");
                    break;
                }
            }
        }

        match get_vehicle_data(&client_clone, car_id).await {
            Ok(data) => {
                if let Err(e) = vehicle_data_tx.send(data) {
                    log::error!("Error sending vehicle data over mpsc: {e}");
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
                        sleep(Duration::from_millis(settings.logging_period_ms as u64)).await;
                        continue;
                    }
                    TeslaError::InvalidHeader(e) => log::error!("Error: `{e}`"),
                    TeslaError::ParseError(e) => log::error!("Error: `{e}`"),
                    TeslaError::WebSocketError(e) => log::error!("Error: `{e}`"),
                    TeslaError::TokenExpired(e) => log::error!("Error: `{e}`"),
                    TeslaError::JsonDecodeError(e) => log::error!("Error: `{e}`"),
                    TeslaError::RequestTimeout => {
                        log::info!("Timeout");
                        // Wait for for a bit before trying again
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                    TeslaError::InvalidResponse => log::error!("Error: `{e}`"),
                }
            }
        };

        _num_data_points += 1;

        // TODO: Send num_data_points to ui server
        // ui_server.lock().await.status.current_points = num_data_points;

        sleep(Duration::from_millis(settings.logging_period_ms as u64)).await;
    }

    log::warn!("Logging stopped");
}

async fn logging_process(
    pool: &sqlx::PgPool,
    client: &TeslaClient,
    tokens: &AuthResponse,
    rx: &mut UnboundedReceiver<bool>,
) -> anyhow::Result<()> {
    let vehicles = get_vehicles(client).await?;
    let vehicle = vehicles.get(0); // TODO: Use the first vehicle for now
    let car_id = vehicle
        .context("Invalid vehicle data")?
        .id
        .context("Invalid ID")?;

    let vehicle_id = vehicle
        .context("Invalid vehicle data")?
        .vehicle_id
        .context("Invalid vehicle ID")?;

    let settings = Settings::db_get_last(pool).await?;

    let access_token = tokens.access_token.clone();

    let client_clone = client.clone();
    let (start_logger_signal_tx, start_logger_signal_rx) = unbounded_channel::<bool>();

    let (vehicle_data_tx, mut vehicle_data_rx) = unbounded_channel::<String>();
    let (streaming_data_tx, streaming_data_rx) = mpsc::channel::<StreamingData>();

    let enable_streaming = false;

    // Start a thread to handle streaming data
    if enable_streaming {
        std::thread::Builder::new()
            .name("data_streaming".to_string())
            .spawn(move || {
                if let Err(e) = stream::start(&access_token, vehicle_id, streaming_data_tx) {
                    log::error!("Error streaming: {e}");
                };
                log::warn!("Vehicle data streaming stopped");
            })?;
    }

    let settings_clone = settings.clone();
    // Start a task to collect vehicle data
    tokio::task::spawn(async move {
        get_vehicle_data_task(
            start_logger_signal_rx,
            client_clone,
            car_id,
            vehicle_data_tx,
            settings_clone,
        )
        .await;
    });

    // Behavior of the logger at startup
    // true  -> begin logging at startup
    // false -> don't begin logging at startup; wait for the user to enable logging.
    if let Err(e) = start_logger_signal_tx.send(settings.log_at_startup) {
        log::error!("Error sending mpsc message to start vehicle data logger: {e}");
    }

    log::info!("Logging started");

    let mut vin_id_map = database::tables::get_vin_id_map(pool).await;
    let mut tables = Tables::default();

    loop {
        if let Ok(value) = rx.try_recv() {
            if let Err(e) = start_logger_signal_tx.send(value) {
                log::error!("Error sending mpsc message to start vehicle data logger: {e}");
            }
        }

        if let Ok(_data) = streaming_data_rx.try_recv() {
            // TODO: inert data into database, create database table for streaming data.
        }

        if let Ok(data) = vehicle_data_rx.try_recv() {
            let data_json = VehicleData::from_response_json(&data);

            if let Err(e) = database::tables::vehicle_data::db_insert_json(&data, pool).await {
                log::error!("{e}");
            };

            match data_json {
                Ok(data) => {
                    (vin_id_map, tables) =
                        database::tables::logging_process(pool, vin_id_map, tables, &data).await;
                }
                Err(e) => log::error!("Error parsing vehicle data to json: {e}"),
            };
        } else {
            sleep(Duration::from_millis(1)).await; // Add a small delay to prevent hogging tokio runtime
        }
    }
}
