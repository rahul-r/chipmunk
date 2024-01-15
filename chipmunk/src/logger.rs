use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Runtime;

use anyhow::Context;
use backend::server::TeslaServer;
use tesla_api::{
    auth::AuthResponse,
    get_tesla_client, get_vehicle_data, get_vehicles,
    stream::{self, StreamingData},
    vehicle_data::VehicleData,
    TeslaClient, TeslaError,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::database::{
    self,
    tables::{
        car::{db_get_or_insert_car, Car},
        charging::ChargingProcess,
        drive::Drive,
        settings::Settings,
        state::State,
    },
};

pub async fn start(
    pool: &sqlx::PgPool,
    server: Arc<Mutex<TeslaServer>>,
    encryption_key: &String,
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
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            continue;
        };

        let tokens = database::token::get(pool, &encryption_key).await?;

        let tesla_client = get_tesla_client(&tokens.access_token)?;

        if let Err(e) = logging_process(pool, &tesla_client, &server, &tokens, &mut rx).await {
            log::error!("Error logging vehicle data: {e}, restarting the logger...");
        } else {
            log::error!("Logging stopped");
            break;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Ok(())
}

async fn logging_process(
    pool: &sqlx::PgPool,
    client: &TeslaClient,
    server: &Arc<Mutex<TeslaServer>>,
    tokens: &AuthResponse,
    rx: &mut UnboundedReceiver<bool>,
) -> anyhow::Result<()> {
    let vehicles = get_vehicles(client).await?;
    let vehicle = vehicles.get(0); // TODO: Use the first vehicle for now
    let id = vehicle
        .context("Invalid vehicle data")?
        .id
        .context("Invalid ID")?;

    let vehicle_id = vehicle
        .context("Invalid vehicle data")?
        .vehicle_id
        .context("Invalid vehicle ID")?;

    let mut num_data_points = 0;
    let settings = Settings::db_get(&pool).await?;

    let access_token = tokens.access_token.clone();

    let server_clone = server.clone();
    let client_clone = client.clone();
    let (start_logger_signal_tx, start_logger_signal_rx) = mpsc::channel::<bool>();

    let (vehicle_data_tx, vehicle_data_rx) = mpsc::channel::<String>();
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

    // Start a thread to collect vehicle data
    std::thread::Builder::new()
        .name("data_logger".to_string())
        .spawn(move || -> anyhow::Result<()> {
            let rt = match Runtime::new() {
                Ok(v) => v,
                Err(e) => {
                    log::error!("Error creating tokio runtime: {e}");
                    anyhow::bail!("Error creating tokio runtime: {e}");
                }
            };

            rt.block_on(async move {
                let mut logging_status = false;
                loop {
                    match start_logger_signal_rx.try_recv() {
                        Ok(v) => {
                            logging_status = v;
                            match server_clone.lock() {
                                Ok(mut srv) => srv.set_logging_status(logging_status),
                                Err(e) => log::error!("{e}"),
                            }
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => (),
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            log::error!("Logger disconnected");
                            break;
                        }
                    }

                    if !logging_status {
                        // Logging is disabled, wait for logging to be enabled
                        match start_logger_signal_rx.recv() {
                            Ok(v) => {
                                logging_status = v;
                                match server_clone.lock() {
                                    Ok(mut srv) => srv.set_logging_status(logging_status),
                                    Err(e) => log::error!("{e}"),
                                }
                            }
                            Err(e) => {
                                log::error!("Logger disconnected: {e}");
                                break;
                            }
                        }
                    }

                    match get_vehicle_data(&client_clone, id).await {
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
                                    tokio::time::sleep(std::time::Duration::from_millis(
                                        settings.logging_period_ms as u64,
                                    ))
                                    .await;
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
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                    continue;
                                }
                                TeslaError::InvalidResponse => log::error!("Error: `{e}`"),
                            }
                        }
                    };

                    num_data_points += 1;

                    match server_clone.lock() {
                        Ok(mut srv) => srv.status.current_points = num_data_points,
                        Err(e) => log::error!("{e}"),
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(
                        settings.logging_period_ms as u64,
                    ))
                    .await;
                    continue;
                }
            });

            log::warn!("Logging stopped");
            Ok(())
        })?;

    // Behavior of the logger at startup
    // true  -> begin logging at startup
    // false -> don't begin logging at startup; wait for the user to enable logging.
    if let Err(e) = start_logger_signal_tx.send(settings.log_at_startup) {
        log::error!("Error sending mpsc message to start vehicle data logger: {e}");
    }

    log::info!("Logging started");

    let mut drive = Drive::db_load_last(pool).await.unwrap_or(Drive::default()); // If there are any errors reading from database, use the default value
    let mut charging_process = ChargingProcess::db_load_last(pool)
        .await
        .unwrap_or(ChargingProcess::default()); // If there are any errors reading from database, use the default value

    let mut state = State::db_load_last(pool).await.unwrap_or(State::default());

    // Read the list of cars from the database, we will check which car the vehicle_data response from the API belongs to
    // It is more efficient to store the list of cars in memory and check against it instead of querying the database for each vehicle_data response
    let mut cars = Car::db_get(pool).await?;

    let mut previous_data: Option<VehicleData> = None;

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
                    // Create database tables from vehicle data
                    match db_get_or_insert_car(pool, cars.clone(), &data).await {
                        Ok((updated_cars, car_id)) => {
                            cars = updated_cars;
                            state.car_id = car_id;
                        }
                        Err(e) => {
                            log::error!("{e}");
                            continue;
                        }
                    };

                    match database::tables::create_tables(
                        pool,
                        &data,
                        previous_data,
                        drive.clone(),
                        charging_process.clone(),
                        state.clone(),
                    )
                    .await
                    {
                        Ok((new_drive, new_charging, new_state)) => {
                            drive = new_drive;
                            charging_process = new_charging;
                            state = new_state;
                        }
                        Err(e) => log::error!("Error adding to database: {e}"),
                    }

                    previous_data = Some(data.clone());
                }
                Err(e) => log::error!("Error parsing vehicle data to json: {e}"),
            };
        }
    }
}
