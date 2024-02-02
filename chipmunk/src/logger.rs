use std::{collections::HashMap, sync::mpsc};

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
        tables::{
            address::Address,
            car::Car,
            car_settings::CarSettings,
            charges::Charges,
            charging_process::ChargingProcess,
            drive::Drive,
            position::Position,
            settings::Settings,
            state::{State, StateStatus},
            token::Token,
            Tables,
        },
        DBTable,
    },
    utils::{sub_option, time_diff},
    EnvVars,
};

pub async fn log(pool: &sqlx::PgPool, env: &EnvVars) -> anyhow::Result<()> {
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
                    if let Err(e) = Token::db_insert(&pool1, tokens, encryption_key.as_str()).await
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
        if !Token::exists(pool).await? {
            if !message_shown {
                log::info!(
                    "Cannot find Tesla auth tokens in database, waiting for token from user"
                );
                message_shown = true;
            }
            sleep(Duration::from_secs(2)).await;
            continue;
        };

        let tokens = Token::db_get_last(pool, encryption_key).await?;

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

    let mut vin_id_map = database::tables::car::get_vin_id_map(pool).await;
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
                        process_vehicle_data(pool, vin_id_map, tables, &data).await;
                }
                Err(e) => log::error!("Error parsing vehicle data to json: {e}"),
            };
        } else {
            sleep(Duration::from_millis(1)).await; // Add a small delay to prevent hogging tokio runtime
        }
    }
}

pub async fn process_vehicle_data(
    pool: &sqlx::PgPool,
    mut vin_id_map: HashMap<String, i16>,
    mut prev_tables: Tables,
    data: &VehicleData,
) -> (HashMap<String, i16>, Tables) {
    let Some(vin) = &data.vin else {
        log::warn!("VIN is None, skipping this entry");
        return (vin_id_map, prev_tables);
    };

    // Check if the vehicle_data response belongs to a car in the database, if not, insert a new entry and update `vin_id_map`
    let car_id = if let Some(id) = vin_id_map.get(vin) {
        *id
    } else {
        log::info!(
            "Vehicle with VIN {} not found in the database, inserting a new entry into database",
            vin
        );
        let car_settings_id = match CarSettings::default().db_insert(pool).await {
            Ok(id) => id,
            Err(e) => {
                log::error!("Error inserting car settings into database: {e}");
                return (vin_id_map, prev_tables);
            }
        };
        let Ok(car) = Car::from(data, car_settings_id).map_err(|e| log::error!("Error creating car: {e}")) else {
            return (vin_id_map, prev_tables);
        };
        // TODO: move to the main db_insert function
        let Ok(id) = car.db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}")).map(|id| id as i16)
        else {
            return (vin_id_map, prev_tables);
        };
        vin_id_map.insert(vin.clone(), id);
        id
    };

    if let Ok(table_list) = create_tables(data, &prev_tables, car_id)
        .await
        .map_err(|e| log::error!("Error adding to database: {e}"))
    {
        for t in table_list {
            match t.db_insert(pool).await {
                Ok(updated_tables) => prev_tables = updated_tables,
                Err(e) => log::error!("Error inserting tables into database: {e}"),
            }
        }
    };

    (vin_id_map, prev_tables)
}

pub async fn create_tables(
    data: &VehicleData,
    prev_tables: &Tables,
    car_id: i16,
) -> anyhow::Result<Vec<Tables>> {
    let current_state = State::from(data, car_id).unwrap();
    let current_position = Position::from(data, car_id, None).unwrap();
    let current_charge = Charges::from(data, 0).map_err(|e| log::error!("{e}")).ok();

    let mut table_list = vec![];

    if let Some(tables) =
        check_hidden_process(car_id, prev_tables, &current_position, &current_charge).await
    {
        table_list = tables;
    } else {
        let (end_prev_state, start_new_state) = current_state.transition(&prev_tables.state);

        // If no state changes, continue logging current state
        if end_prev_state.is_none() && start_new_state.is_none() {
            table_list.push(
                continue_logging(prev_tables, current_state, current_position, current_charge)
                    .await,
            );
        } else {
            if let Some(prev_state) = end_prev_state {
                table_list
                    .push(end_logging_for_state(prev_state, prev_tables, &current_charge).await);
            }
            if let Some(new_state) = start_new_state {
                table_list.push(
                    start_logging_for_state(new_state, car_id, current_position, current_charge)
                        .await,
                );
            }
        }
    }

    Ok(table_list)
}

async fn continue_logging(
    prev_tables: &Tables,
    current_state: State,
    current_position: Position,
    current_charge: Option<Charges>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut drive: Option<Drive> = None;

    let state = current_state.state;
    match state {
        S::Driving => {
            drive = prev_tables
                .drive
                .as_ref()
                .map(|d| d.update(&current_position))
        }
        S::Offline => (),
        S::Asleep => (),
        S::Unknown => todo!(),
        S::Parked => (),
        S::Charging => {
            charging_process = prev_tables
                .charging_process
                .as_ref()
                .zip(current_charge.as_ref())
                .map(|(cp, c)| cp.update(c))
        }
    }

    let position = match state.is_online() {
        true => Some(Position {
            drive_id: drive.as_ref().map(|d| d.id),
            ..current_position
        }),
        false => None,
    };

    let state = Some(State {
        end_date: current_position.date,
        ..prev_tables.state.clone().unwrap_or_default()
    });

    Tables {
        drive,
        address: None,
        car: None,
        charges: current_charge,
        charging_process,
        position,
        settings: None,
        state,
        sw_update: None,
    }
}

async fn end_logging_for_state(
    state: StateStatus,
    prev_tables: &Tables,
    current_charge: &Option<Charges>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut drive: Option<Drive> = None;

    match state {
        S::Driving => {
            drive = prev_tables
                .drive
                .as_ref()
                .zip(prev_tables.position.as_ref())
                .map(|(d, pos)| d.stop(&pos, None, None))
        }
        S::Charging => {
            charging_process = prev_tables
                .charging_process
                .as_ref()
                .zip(current_charge.as_ref())
                .map(|(cp, c)| cp.update(c))
        }
        S::Asleep => (),
        S::Offline => (),
        S::Unknown => todo!(),
        S::Parked => (),
    }

    // Insert address only if we are ending a drive
    let address = match drive.as_ref().zip(prev_tables.position.as_ref()) {
        Some((_, p)) => Address::from_opt(p.latitude, p.longitude)
            .await
            .map_err(|e| log::error!("Error getting address: {e}"))
            .ok(),
        None => None,
    };

    let position = match state.is_online() {
        true => prev_tables.position.clone(),
        false => None,
    };

    let state = Some(State {
        end_date: prev_tables.position.as_ref().map(|p| p.date).flatten(),
        ..prev_tables.state.clone().unwrap_or_default()
    });

    Tables {
        address,
        car: None,
        charges: current_charge.clone(),
        charging_process,
        drive: drive.clone(),
        position,
        settings: None,
        state,
        sw_update: None,
    }
}

async fn start_logging_for_state(
    new_state: StateStatus,
    car_id: i16,
    current_position: Position,
    current_charge: Option<Charges>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut drive: Option<Drive> = None;

    match new_state {
        S::Driving => drive = Some(Drive::start(&current_position, car_id, None, None)),
        S::Charging => {
            charging_process = current_charge
                .as_ref()
                .map(|c| ChargingProcess::start(c, car_id, 0, None, None));
        }
        S::Asleep => (),
        S::Offline => (),
        S::Unknown => todo!(),
        S::Parked => (),
    }

    let address = if drive.is_some() || charging_process.is_some() {
        Address::from_opt(current_position.latitude, current_position.longitude)
            .await
            .map_err(|e| log::error!("Error getting address: {e}"))
            .ok()
    } else {
        None
    };

    let state = Some(State {
        car_id,
        state: new_state,
        start_date: current_position.date.unwrap_or_else(|| {
            log::error!("Timestamp is None, using current time");
            chrono::Utc::now().naive_utc()
        }),
        ..State::default()
    });

    let position = match new_state.is_online() {
        true => Some(current_position),
        false => None,
    };

    Tables {
        address,
        car: None,
        charges: current_charge,
        charging_process,
        drive,
        position,
        settings: None,
        state,
        sw_update: None,
    }
}

/// This function checks for hidden drive or charge processes and returns a Vec of Tables
/// representing the detected processes.
///
/// ## Hidden drive detection:
/// If the time difference between the previous and current data points is more than 10 minutes
/// and the vehicle has not moved more than 1 mile since the previous position,
/// it ends the current driving state and starts a new one.
///
/// ## Hidden charging process detection:
/// This function checks if the vehicle was charged since the previous data point was received
/// by checking if the vehicle was moved since the previous data point and the battery level
/// is up by more than 1% (The 1% check is to avoid logging any tolerance in charge level detection
/// as a new charging process).
///
/// # Arguments
///
/// * `car_id` - The ID of the car.
/// * `prev_tables` - A reference to the previous `Tables`.
/// * `current_position` - A reference to the current `Position`.
/// * `current_charge` - An `Option` that holds a reference to the current `Charges`.
///
/// # Returns
///
/// * `Option<Vec<Tables>>` - Returns `None` if no hidden drive or charge sessions are detected.
async fn check_hidden_process(
    car_id: i16,
    prev_tables: &Tables,
    current_position: &Position,
    current_charge: &Option<Charges>,
) -> Option<Vec<Tables>> {
    let previous_state = prev_tables.state.clone();
    let drive = prev_tables.drive.clone();
    let mut table_list = vec![];

    // End the previous state and start a new state if the previous data point was more than 10 minutes ago
    // and the vehicle has not moved since then.
    if time_diff(prev_tables.get_time(), current_position.date)
        > Some(chrono::Duration::minutes(10))
    {
        // previous data point was more than 10 minutes ago
        if let Some(ref prev_position) = prev_tables.position {
            if sub_option(current_position.odometer, prev_position.odometer) < Some(1.0) {
                // vehicle has not moved since the previous data point

                // Since the vehicle has not moved, previous ans current positions will give the same address
                // Using current position so we don't need to deal with Option<>
                let address =
                    Address::from_opt(current_position.latitude, current_position.longitude)
                        .await
                        .map_err(|e| log::error!("Error getting address: {e}"))
                        .ok();

                // End the current drive
                let state = State {
                    end_date: prev_tables.get_time(),
                    ..previous_state.clone().unwrap_or_default()
                };
                table_list.push(Tables {
                    address: address.clone(),
                    drive: drive.map(|d| d.stop(current_position, None, None)),
                    position: Some(current_position.clone()),
                    state: Some(state),
                    ..Default::default()
                });

                // Check if the vehicle was charged since the previous data point
                let prev_battery_level =
                    prev_tables.position.as_ref().and_then(|p| p.battery_level);
                let curr_battery_level = current_position.battery_level;
                if let Some(current_charge) = current_charge {
                    if let Some(charge) = sub_option(curr_battery_level, prev_battery_level) {
                        // Continue only if the battery level us up by at least 2% (>1)
                        if charge > 1 {
                            // Create a new charging process
                            ChargingProcess::from_charges(
                                prev_tables.charges.as_ref(),
                                current_charge,
                                car_id,
                                current_position.id.unwrap_or(0),
                                address.as_ref().map(|a| a.id as i32),
                                None,
                            )
                            .map_err(|e| log::error!("Error creating charging process: {e}"))
                            .map(|cp| {
                                // Tables for beginning of charging
                                table_list.push(Tables {
                                    address: address.clone(),
                                    charging_process: Some(cp.clone()),
                                    charges: prev_tables.charges.clone(),
                                    position: Some(prev_position.clone()),
                                    ..Default::default()
                                });
                                // Tables for end of charging
                                table_list.push(Tables {
                                    charging_process: Some(cp),
                                    charges: Some(current_charge.clone()),
                                    position: Some(current_position.clone()),
                                    state: Some(State {
                                        state: StateStatus::Charging,
                                        start_date: prev_tables.get_time().unwrap_or_default(),
                                        end_date: current_position.date,
                                        car_id,
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                })
                            })
                            .ok();
                        }
                    }
                }

                // Start a new drive
                let new_drive = Drive {
                    start_position_id: prev_position.id,
                    ..Drive::start(current_position, car_id, None, None)
                };
                let state = Some(State {
                    state: StateStatus::Driving,
                    start_date: current_position.date.unwrap_or_default(),
                    car_id,
                    ..Default::default()
                });
                table_list.push(Tables {
                    address,
                    drive: Some(new_drive),
                    position: Some(current_position.clone()),
                    state,
                    ..Default::default()
                });
            }
        }
        return Some(table_list);
    }
    None
}
