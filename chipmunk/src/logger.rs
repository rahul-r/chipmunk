use std::collections::HashMap;

use tesla_api::vehicle_data::VehicleData;

use crate::{
    database::{
        tables::{
            address::Address,
            car::Car,
            car_settings::CarSettings,
            charges::Charges,
            charging_process::ChargingProcess,
            drive::Drive,
            position::Position,
            state::{State, StateStatus},
            Tables,
        },
        types::ChargeStat,
        DBTable,
    },
    tasks,
    utils::sub_option,
};

pub async fn log(pool: &sqlx::PgPool, env: &crate::EnvVars) -> anyhow::Result<()> {
    tasks::run(&env, &pool).await
}

pub async fn process_vehicle_data(
    pool: &sqlx::PgPool,
    mut vin_id_map: HashMap<String, i16>,
    prev_tables: Tables,
    data: VehicleData,
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
        let Ok(car) =
            Car::from(&data, car_settings_id).map_err(|e| log::error!("Error creating car: {e}"))
        else {
            return (vin_id_map, prev_tables);
        };
        // TODO: move to the main db_insert function
        let Ok(id) = car
            .db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}"))
            .map(|id| id as i16)
        else {
            return (vin_id_map, prev_tables);
        };
        vin_id_map.insert(vin.clone(), id);
        id
    };

    let mut last_inserted_tables: Option<Tables> = None;
    if let Ok(table_list) = create_tables(&data, &prev_tables, car_id)
        .await
        .map_err(|e| log::error!("Error adding to database: {e}"))
    {
        for t in table_list {
            match t.db_insert(pool).await {
                Ok(updated_tables) => last_inserted_tables = Some(updated_tables),
                Err(e) => log::error!("Error inserting tables into database: {e}"),
            }
        }
    };

    (vin_id_map, last_inserted_tables.unwrap_or(prev_tables))
}

pub async fn create_tables(
    data: &VehicleData,
    prev_tables: &Tables,
    car_id: i16,
) -> anyhow::Result<Vec<Tables>> {
    let current_state = State::from(data, car_id)?;
    let current_position = Position::from(data, car_id, None)?;
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
            let t = continue_logging(prev_tables, current_state, current_position, current_charge)
                .await;
            table_list.push(t);
        } else {
            if let Some(prev_state) = end_prev_state {
                let t = end_logging_for_state(
                    prev_state,
                    prev_tables,
                    &current_position,
                    current_charge.clone(),
                    None,
                )
                .await;
                table_list.push(t);
            }
            if let Some(new_state) = start_new_state {
                let t = start_logging_for_state(
                    new_state,
                    car_id,
                    current_position,
                    current_charge,
                    None,
                )
                .await;
                table_list.push(t);
            }
        }
    }

    // Insert raw vehicle data into the last table
    if let Some(t) = table_list.last_mut() {
        t.raw_data = Some(data.clone());
    }

    Ok(table_list)
}

async fn start_logging_for_state(
    new_state: StateStatus,
    car_id: i16,
    current_position: Position,
    current_charge: Option<Charges>,
    address_override: Option<Address>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut charges: Option<Charges> = None;
    let mut drive: Option<Drive> = None;

    match new_state {
        S::Driving => drive = Some(Drive::start(&current_position, car_id, None, None)),
        S::Charging => {
            charging_process = current_charge
                .as_ref()
                .map(|c| ChargingProcess::start(c, car_id, 0, None, None));
            if charging_process.is_some() {
                charges = current_charge;
            }
        }
        S::Asleep | S::Offline | S::Unknown | S::Parked => (),
    }

    let address = if drive.is_some() || charging_process.is_some() {
        if address_override.is_some() {
            address_override
        } else {
            Address::from_opt(current_position.latitude, current_position.longitude)
                .await
                .map_err(|e| log::error!("Error getting address: {e}"))
                .ok()
        }
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

    let time = current_position.date;
    let position = match new_state.is_online() {
        true => Some(current_position),
        false => None,
    };

    Tables {
        address,
        car: None,
        charges,
        charging_process,
        drive,
        position,
        settings: None,
        state,
        sw_update: None,
        time,
        raw_data: None,
    }
}

async fn continue_logging(
    prev_tables: &Tables,
    current_state: State,
    current_position: Position,
    current_charge: Option<Charges>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut charges: Option<Charges> = None;
    let mut drive: Option<Drive> = None;

    let state = current_state.state;
    match state {
        S::Driving => {
            drive = prev_tables
                .drive
                .as_ref()
                .map(|d| d.update(&current_position))
        }
        S::Charging => {
            charging_process = prev_tables
                .charging_process
                .as_ref()
                .zip(current_charge.as_ref())
                .map(|(cp, c)| cp.update(c));
            if charging_process.is_some() {
                charges = current_charge;
            }
        }
        S::Asleep | S::Offline | S::Unknown | S::Parked => (),
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
        charges,
        charging_process,
        position,
        settings: None,
        state,
        sw_update: None,
        time: current_position.date,
        raw_data: None,
    }
}

async fn end_logging_for_state(
    state: StateStatus,
    prev_tables: &Tables,
    curr_position: &Position,
    curr_charge: Option<Charges>,
    address_override: Option<Address>,
) -> Tables {
    use StateStatus as S;

    let mut charging_process: Option<ChargingProcess> = None;
    let mut charges: Option<Charges> = None;
    let mut drive: Option<Drive> = None;

    match state {
        S::Driving => {
            drive = prev_tables
                .drive
                .as_ref()
                .zip(prev_tables.position.as_ref())
                .map(|(d, pos)| d.stop(pos, None, None))
        }
        S::Charging => {
            charging_process = prev_tables
                .charging_process
                .as_ref()
                .zip(curr_charge.as_ref())
                .map(|(cp, c)| cp.update(c))
                .map(|mut cp| {
                    cp.charging_status = ChargeStat::Done;
                    cp
                });
            if charging_process.is_some() {
                charges = curr_charge;
            }
        }
        S::Asleep | S::Offline | S::Unknown | S::Parked => (),
    }

    // Insert address only if we are ending a drive
    let address = match drive.as_ref().zip(prev_tables.position.as_ref()) {
        Some((_, p)) => {
            if address_override.is_some() {
                address_override
            } else {
                Address::from_opt(p.latitude, p.longitude)
                    .await
                    .map_err(|e| log::error!("Error getting address: {e}"))
                    .ok()
            }
        }
        None => None,
    };

    let position = match state.is_online() {
        true => prev_tables.position.clone(),
        false => None,
    };

    let state = Some(State {
        end_date: prev_tables.get_time(),
        ..prev_tables.state.clone().unwrap_or_default()
    });

    Tables {
        address,
        car: None,
        charges,
        charging_process,
        drive: drive.clone(),
        position,
        settings: None,
        state,
        sw_update: None,
        time: curr_position.date,
        raw_data: None,
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
    curr_position: &Position,
    curr_charge: &Option<Charges>,
) -> Option<Vec<Tables>> {
    let mut table_list = vec![];

    // Continue only if the previous state was either Driving
    if !prev_tables.is_driving() {
        return None;
    }

    // End the previous state and start a new state if the previous data point was more than 10 minutes ago
    // and the vehicle has not moved since then.
    if prev_tables
        .get_time()
        .zip(curr_position.date)
        .map(|(prev, curr)| curr - prev)
        .map(|diff| diff <= chrono::Duration::try_minutes(10).expect("This should always pass"))
        .unwrap_or(true)
    {
        return None;
    }

    let Some(ref prev_position) = prev_tables.position else {
        // No previous position to compare with
        return None;
    };

    if sub_option(curr_position.odometer, prev_position.odometer) >= Some(1.0) {
        // vehicle has moved since the previous data point
        return None;
    }

    // Since the vehicle has not moved, previous and current positions will give the same address
    // Using current position, so we don't need to deal with Option<>
    let address = Address::from_opt(curr_position.latitude, curr_position.longitude)
        .await
        .map_err(|e| log::error!("Error getting address: {e}"))
        .ok();

    if prev_tables.is_driving() {
        // End the current drive
        let t = end_logging_for_state(
            StateStatus::Driving,
            prev_tables,
            curr_position,
            curr_charge.clone(),
            address.clone(),
        )
        .await;
        table_list.push(t);
    }

    // Check if the vehicle was charged since the previous data point
    let prev_battery_level = prev_tables.position.as_ref().and_then(|p| p.battery_level);
    let curr_battery_level = curr_position.battery_level;

    let prev_charge = match prev_tables.raw_data.as_ref().map(|d| Charges::from(d, 0)) {
        Some(Ok(c)) => Some(c),
        Some(Err(e)) => {
            log::error!("Error creating charges from previous vehicle data: {e}");
            None
        }
        None => {
            log::error!(
                "`prev_tables.raw_data` is None, cannot create charges from `prev_tables.raw_data`"
            );
            None
        }
    };

    const MIN_BATT_GAIN_TO_LOG: i16 = 1;

    if let (Some(current_charge), Some(prev_charge)) = (curr_charge, prev_charge) {
        let log_charging = prev_battery_level
            .zip(curr_battery_level)
            .map(|(p, c)| c - p)
            .map(|diff| diff  > MIN_BATT_GAIN_TO_LOG) // Continue only if the battery level us up by more than MIN_BATT_GAIN_TO_LOG
            .inspect(|d| if !d { log::info!("Battery is not charged at least {MIN_BATT_GAIN_TO_LOG}%, skipping charging process creation"); })
            .unwrap_or_else(|| { log::warn!("Missing previous and/or current battery levels, skipping charging process creation"); false});
        if log_charging {
            let charging_process = ChargingProcess::from_charges(
                &prev_charge,
                current_charge,
                car_id,
                curr_position.id.unwrap_or(0),
                address.as_ref().map(|a| a.id as i32),
                None,
            )
            .map_err(|e| log::error!("Error creating charging process: {e}"))
            .ok();

            if let Some(cp) = charging_process {
                // Tables for charging
                table_list.push(Tables {
                    address: address.clone(),
                    charging_process: Some(cp),
                    charges: Some(prev_charge),
                    position: Some(prev_position.clone()),
                    time: prev_tables.get_time(),
                    state: Some(State {
                        id: 0,
                        state: StateStatus::Charging,
                        start_date: prev_tables.get_time().unwrap_or_default(),
                        end_date: curr_position.date,
                        car_id,
                    }),
                    ..Default::default()
                });
                // Tables to log current_charge (prev_charge was included in the table above)
                table_list.push(Tables {
                    charges: Some(current_charge.clone()),
                    time: curr_position.date,
                    ..Default::default()
                })
            }
        }
    }

    // Start a new drive
    let t = start_logging_for_state(
        StateStatus::Driving,
        car_id,
        curr_position.clone(),
        curr_charge.clone(),
        address,
    )
    .await;
    table_list.push(t);

    Some(table_list)
}
