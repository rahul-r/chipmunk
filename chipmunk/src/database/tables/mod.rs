use std::{cmp::Ordering, collections::HashMap};

use anyhow::Context;
use chrono::{Duration, NaiveDateTime, Utc};
use futures::{future::BoxFuture, FutureExt};
use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use crate::{
    database::tables::car::CarSettings,
    utils::{sub_option, time_diff},
};

use self::{
    address::Address,
    car::Car,
    charging::{handle_charging, Charges, ChargingProcess},
    drive::{handle_drive, Drive, DriveStatus},
    position::Position,
    settings::Settings,
    state::{handle_state_change, State},
    swupdate::{software_updated, SoftwareUpdate},
};

pub mod address;
pub mod car;
pub mod charging;
pub mod charging_db;
pub mod drive;
pub mod drive_db;
pub mod geofence;
pub mod position;
pub mod settings;
pub mod state;
pub mod swupdate;
pub(crate) mod types;
pub mod vehicle_data;

#[derive(Default, Debug, Clone)]
pub struct Tables {
    pub address: Option<Address>,
    pub car: Option<Car>,
    pub charges: Option<Charges>,
    pub charging_process: Option<ChargingProcess>,
    pub drive: Option<Drive>,
    pub position: Option<Position>,
    pub settings: Option<Settings>,
    pub state: Option<State>,
    pub sw_update: Option<SoftwareUpdate>,
}

impl Tables {
    pub fn time(&self) -> Option<NaiveDateTime> {
        #[rustfmt::skip]
        self.position.as_ref().and_then(|p| p.date)
            .or_else(|| self.drive.as_ref().map(|d| d.start_date))
            .or_else(|| self.charging_process.as_ref().map(|cp| cp.start_date))
            .or_else(|| self.state.as_ref().map(|s| s.start_date))
            .or_else(|| self.sw_update.as_ref().map(|sw| sw.start_date))
            .or_else(|| self.address.as_ref().map(|a| a.inserted_at))
            .or_else(|| self.settings.as_ref().map(|s| s.inserted_at))
            .or_else(|| self.car.as_ref().map(|c| c.inserted_at))
    }
}

#[derive(Debug)]
enum RetType {
    SwUpdate(i32),
    Drive(Drive),
    Charging(ChargingProcess),
    State(State),
}

#[macro_export]
macro_rules! convert_result {
    ($err_msg:expr) => {
        |result| async {
            match result {
                Ok(r) => Ok(RetType::Id(r)),
                Err(e) => {
                    if $err_msg == true {
                        log::error!("{e}");
                    }
                    Err(anyhow::anyhow!(e))
                }
            }
        }
    };
}

pub async fn initialize(pool: &PgPool) -> anyhow::Result<()> {
    settings::initialize(pool).await?;
    Ok(())
}

/**
 * Check if timestamp in current data is newer than previous_data timestamp
 * Return Ok(true) if current_data timestamp is newer than previous_data timestamp
*/
fn data_time(prev_data: &Option<VehicleData>, curr_data: &VehicleData) -> anyhow::Result<Ordering> {
    let current_timestamp = &curr_data
        .drive_state
        .as_ref()
        .context("drive_state is None")?
        .timestamp
        .context("timestamp is None")?;
    if let Some(ref prev) = prev_data {
        let previous_timestamp = &prev
            .drive_state
            .as_ref()
            .context("drive_state is None")?
            .timestamp
            .context("timestamp is None")?;

        return Ok(current_timestamp.cmp(previous_timestamp));
    }

    // previous_data = None means the first data point. Return "Ordering::Greater" to indicate
    // this is a good data point
    Ok(Ordering::Greater)
}

/**
 * Convert database form vehicle_data JSON to grafana compatible tables
*/
pub async fn convert_database(
    pool: &PgPool,
    car_data_pool: &PgPool,
    num_rows_to_fetch: i64,
) -> anyhow::Result<()> {
    log::info!("Converting database");

    // TODO: load these from the database at startup
    let mut charging_process = ChargingProcess::default();
    let mut drive = Drive::default();
    let mut previous_data: Option<VehicleData> = None;

    // Read the list of cars from the database, we will check which car the vehicle_data response from the API belongs to
    // It is more efficient to store the list of cars in memory and check against it instead of querying the database for each vehicle_data response
    let mut cars = Car::db_get(pool).await?;

    let num_rows = vehicle_data::num_car_data_rows(car_data_pool).await?;
    let batch_size = if num_rows_to_fetch < 10_000 {
        num_rows_to_fetch
    } else {
        10_000
    };
    let mut row_offset = num_rows - num_rows_to_fetch;
    let mut state = State::default();

    // TODO: Remove the commented code. It is used to test charging process
    // To perform testing, uncomment the following line and comment the following while loop
    // let vehicle_data_list =
    //     vehicle_data::get_car_data_between(car_data_pool, 1700841600000, 1700850000000).await?;
    while (row_offset - batch_size) < num_rows {
        let vehicle_data_list =
            vehicle_data::get_car_data(car_data_pool, batch_size, row_offset).await?;
        for current_data in vehicle_data_list {
            match data_time(&previous_data, &current_data)? {
                Ordering::Less => {
                    // Current data point is before the previous point. Log an error and go back
                    // without processing the data
                    log::warn!("Current timestamp is older than previous timestamp");
                    previous_data = Some(current_data.clone());
                    continue;
                }
                Ordering::Equal => continue, // Timestamps of current and previous data are same. Go back without processing the data
                Ordering::Greater => (),     // Good data, continue processing
            }

            match car::db_get_or_insert_car(pool, cars.clone(), &current_data).await {
                Ok((updated_cars, car_id)) => {
                    cars = updated_cars;
                    state.car_id = car_id;
                }
                Err(e) => {
                    log::error!("{e}");
                    continue;
                }
            };

            match create_tables(
                pool,
                &current_data,
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

            previous_data = Some(current_data.clone());
        }

        row_offset += batch_size;
    }

    Ok(())
}

// async fn calculate_efficiency(pool: &PgPool) -> anyhow::Result<f32> {
//     // TODO: Get preferred_range from settings table
//     let preferred_range = Range::Rated;
//     let charging_processes = ChargingProcess::db_get(pool).await?;
//     let car_id = 0;

//     fn calculate(cp: ChargingProcess, car_id: i16, preferred_range: Range) -> anyhow::Result<f32> {
//         let start_range = match preferred_range {
//             Range::Ideal => cp.start_rated_range_km,
//             Range::Rated => cp.start_ideal_range_km,
//         };
//         let end_range = match preferred_range {
//             Range::Ideal => cp.end_rated_range_km,
//             Range::Rated => cp.end_ideal_range_km,
//         };

//         if cp.car_id == car_id
//             && cp.duration_min.context("unexpected duration")? > 10
//             && cp.end_battery_level.context("unexpected battery level")? > 95
//             && cp.charge_energy_added.context("unexpected charge energy")? > 0.0
//         {
//             if let (Some(energy), Some(end_range), Some(start_range)) =
//                 (cp.charge_energy_added, end_range, start_range)
//             {
//                 return Ok(energy / (end_range - start_range));
//             }
//         }

//         anyhow::bail!("Cannot calculate efficiency");
//     }

//     for cp in charging_processes {
//         match calculate(cp, car_id, preferred_range) {
//             Ok(efficiency) => return Ok(efficiency),
//             Err(e) => log::error!("{e}"),
//         }
//     }

//     anyhow::bail!("Cannot calculate efficiency");
// }

pub async fn create_tables(
    pool: &PgPool,
    curr_data: &VehicleData,
    prev_data: Option<VehicleData>,
    mut drive: Drive,
    mut charging_process: ChargingProcess,
    mut state: State,
) -> anyhow::Result<(Drive, ChargingProcess, State)> {
    // let settings = Settings::default();
    // let geofence = Geofence {
    //     name: "Home".into(),
    //     latitude: 36.2,
    //     longitude: 72.4,
    //     radius: 143,
    //     ..Geofence::default()
    // };
    let mut position = Position::from(curr_data, state.car_id, None).ok();
    let charges = Charges::from(curr_data, 0).ok();
    let mut sw_update = SoftwareUpdate {
        id: 0,
        start_date: Utc::now().naive_utc(),
        end_date: None,
        version: curr_data
            .vehicle_state
            .as_ref()
            .expect("should never happen")
            .car_version
            .as_ref()
            .expect("should never happen")
            .clone(),
        car_id: state.car_id,
    };

    let mut tasks: Vec<BoxFuture<anyhow::Result<RetType>>> = vec![
    //     Box::pin(settings.db_insert(pool).then(convert_result!(false))),
    //     Box::pin(database::insert_geofence(pool, &geofence).then(convert_result!(false))),
    ];

    let insert_state_task =
        handle_state_change(pool, &prev_data, curr_data, &state).then(|result| async {
            match result {
                Ok(s) => Ok(RetType::State(s)),
                Err(e) => {
                    log::error!("{e}");
                    anyhow::bail!(e);
                }
            }
        });
    tasks.push(Box::pin(insert_state_task));

    if software_updated(prev_data.as_ref(), curr_data) {
        // TODO: update the end_date row on the previous software version row with date of
        // previous_data
        sw_update.end_date = position.clone().and_then(|p| p.date);
        if let Err(e) = swupdate::insert_end_date(pool, sw_update).await {
            log::error!("Error updating end date of software version: {e}");
        }

        sw_update = SoftwareUpdate {
            id: 0, // Start with id = 0; we will update the id after inserting to database
            start_date: Utc::now().naive_utc(),
            end_date: None,
            version: curr_data
                .vehicle_state
                .as_ref()
                .expect("should never happen")
                .car_version
                .as_ref()
                .expect("should never happen")
                .clone(),
            car_id: state.car_id,
        };

        let insert_update_task = swupdate::insert(pool, &sw_update).then(|result| async {
            match result {
                Ok(r) => Ok(RetType::SwUpdate(r)),
                Err(e) => {
                    log::error!("{e}");
                    anyhow::bail!(e);
                }
            }
        });
        tasks.push(Box::pin(insert_update_task));
    }

    if let Some(ref mut current_position) = position {
        current_position.car_id = state.car_id;

        let previous_position = match prev_data {
            Some(ref d) => Position::from(d, state.car_id, None).ok(),
            None => None,
        };

        let current_shift = curr_data.drive_state.clone().and_then(|ds| ds.shift_state);

        let handle_drive_task = handle_drive(
            pool,
            previous_position,
            current_position,
            current_shift,
            drive.clone(),
            state.car_id,
        )
        .then(|result| async { Ok(RetType::Drive(result)) });

        tasks.push(Box::pin(handle_drive_task));

        let previous_charge = match prev_data {
            Some(ref d) => Some(Charges::from(d, 0)?),
            None => None,
        };

        if let Some(charges) = charges {
            let handle_charging_task = handle_charging(
                pool,
                curr_data,
                previous_charge,
                charges,
                charging_process.clone(),
                current_position,
                state.car_id,
            )
            .then(|result| async {
                match result {
                    Ok(r) => Ok(RetType::Charging(r)),
                    Err(e) => {
                        log::error!("{e}");
                        Err(e)
                    }
                }
            });
            tasks.push(Box::pin(handle_charging_task));
        }
    }

    let results = futures::future::join_all(tasks).await;

    for result in results.into_iter().flatten() {
        match result {
            RetType::SwUpdate(id) => sw_update.id = id,
            RetType::Drive(d) => drive = d,
            RetType::Charging(c) => charging_process = c,
            RetType::State(s) => state.id = s.id,
        }
    }

    Ok((drive, charging_process, state))
}

fn create_charging_process(
    prev_tables: &Tables,
    data: &VehicleData,
) -> anyhow::Result<ChargingProcess> {
    // Create a new charging process
    let prev_charge = prev_tables.charges.clone();
    let curr_charge = Charges::from(data, 0)?;
    ChargingProcess::from_charges(
        prev_charge.as_ref(),
        &curr_charge,
        prev_tables.state.as_ref().context("State is None")?.car_id,
        prev_tables
            .position
            .as_ref()
            .context("Position is None")?
            .id
            .context("Position ID is none")?,
        None,
        None,
    )
}

pub async fn create_tables_new(
    data: &VehicleData,
    prev_tables: &Tables,
    car_id: i16,
) -> anyhow::Result<Vec<Tables>> {
    let current_state = State::from(data, car_id)?;
    let current_position = Position::from(data, car_id, None)?;
    let mut drive = prev_tables.drive.clone();
    let previous_state = prev_tables.state.clone();

    let mut table_list = vec![];

    // End the previous state and start a new state if the previous data point was more than 10 minutes ago
    // and the vehicle has not moved since then.
    if time_diff(prev_tables.time(), current_position.date) > Some(Duration::minutes(10)) {
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
                    end_date: prev_tables.time(),
                    ..previous_state.unwrap_or_default()
                };
                table_list.push(Tables {
                    address: address.clone(),
                    drive: drive.map(|d| d.stop(&current_position, None, None)),
                    position: Some(current_position.clone()),
                    state: Some(state),
                    ..Default::default()
                });

                // Start a new drive
                table_list.push(Tables {
                    address: address.clone(),
                    drive: Some(Drive::start(&current_position, car_id, None, None)),
                    position: None, // The position has already been logged with previous drive end, no need to log again, use the previously entered position ID to start this drive
                    state: Some(current_state),
                    ..Default::default()
                });

                // Check if the vehicle was charged since the previous data point
                let prev_battery_level =
                    prev_tables.position.as_ref().and_then(|p| p.battery_level);
                let curr_battery_level = current_position.battery_level;
                if let Some(charge) = sub_option(curr_battery_level, prev_battery_level) {
                    // Continue only if the battery level us up by at least 2% (>1)
                    if charge > 1 {
                        // Create a new charging process
                        if let Ok(charging_process) = create_charging_process(prev_tables, data)
                            .map_err(|e| log::error!("Error creating charging process: {e}"))
                        {
                            table_list.push(Tables {
                                address,
                                charging_process: Some(charging_process),
                                position: Some(prev_position.clone()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        return Ok(table_list);
    }

    let (end_prev_state, start_new_state) = current_state.transition(&previous_state);
    // If no state changes, continue logging current state
    if end_prev_state.is_none() && start_new_state.is_none() {
        use state::StateStatus::*;
        let state = Some(State {
            end_date: current_position.date,
            ..previous_state.clone().unwrap_or_default()
        });
        let mut position: Option<Position> = None;
        match current_state.state {
            Driving => {
                drive = drive.map(|d| d.update(&current_position));
                position = Some(current_position.clone());
            }
            Offline => todo!(),
            Asleep => todo!(),
            Unknown => todo!(),
            Parked => drive = None,
            Charging => todo!(),
        }

        table_list.push(Tables {
            drive: drive.clone(),
            address: None, // Don't log address with every drive status update to reduce strain on openstreetmap API
            car: None,
            charges: None,
            charging_process: None,
            position,
            settings: None,
            state,
            sw_update: None,
            // ..prev_tables.clone()
        });
    }

    if let Some(prev_state) = end_prev_state {
        use state::StateStatus::*;
        let mut address: Option<Address> = None;
        let mut position = Some(current_position.clone());
        let state = Some(State {
            end_date: current_position.date,
            ..previous_state.unwrap_or_default()
        });
        match prev_state {
            Driving => {
                drive = drive.map(|d| d.stop(&current_position, None, None));
                address = Address::from_opt(current_position.latitude, current_position.longitude)
                    .await
                    .map_err(|e| log::error!("Error getting address: {e}"))
                    .ok();
            }
            Charging => todo!(),
            Asleep => todo!(),
            Offline => todo!(),
            Unknown => todo!(),
            Parked => position = None,
        }

        table_list.push(Tables {
            address: address.clone(),
            car: None,
            charges: None,
            charging_process: None,
            drive: drive.clone(),
            position,
            settings: None,
            state,
            sw_update: None,
        });
    }

    if let Some(new_state) = start_new_state {
        use state::StateStatus::*;
        let mut address: Option<Address> = None;
        let mut position = Some(current_position.clone());
        let state = Some(State {
            car_id,
            state: new_state,
            start_date: current_position.date.unwrap_or_else(|| {
                log::error!("Timestamp is None, using current time");
                Utc::now().naive_utc()
            }),
            ..State::default()
        });
        match new_state {
            Driving => {
                drive = Some(Drive::start(&current_position, car_id, None, None));
                address = Address::from_opt(current_position.latitude, current_position.longitude)
                    .await
                    .map_err(|e| log::error!("Error getting address: {e}"))
                    .ok();
            }
            Charging => todo!(),
            Asleep => todo!(),
            Offline => todo!(),
            Unknown => todo!(),
            Parked => {
                position = None;
                drive = None;
            }
        }

        table_list.push(Tables {
            address: address.clone(),
            car: None,
            charges: None,
            charging_process: None,
            drive: drive.clone(),
            position,
            settings: None,
            state,
            sw_update: None,
        });
    }

    Ok(table_list)
}

pub async fn db_insert(pool: &sqlx::PgPool, tables: Tables) -> anyhow::Result<Tables> {
    let mut tables = tables;

    // // Insert state table
    // if let Some(ref mut s) = tables.state {
    //     s.db_insert(&pool)
    //         .await
    //         .and_then(|id| Ok(s.id = id))?;
    // }

    // Insert position and update the ID field
    if let Some(ref mut p) = tables.position {
        p.db_insert(pool).await.map(|id| p.id = Some(id as i32))?; // Update id field of current_position with the id returned from the database
    }

    // Insert address and update the ID field
    let address_id = if let Some(ref address) = tables.address {
        address
            .db_insert(pool)
            .await
            .map_err(|e| log::error!("Error inserting address into database: {e}"))
            .map(|id| id as i32)
            .ok()
    } else {
        None
    };

    if let Some(ref mut drive) = tables.drive {
        if address_id.is_some() {
            if drive.status == DriveStatus::Driving {
                drive.start_address_id = address_id;
            } else {
                drive.end_address_id = address_id;
            }
        }

        // let start_geofence_id = None; // TODO: Add this
        // let end_geofence_id = None; // TODO: Add this

        // If starting a new drive
        if drive.id == 0 {
            if drive.start_position_id.is_none() {
                drive.start_position_id = tables.position.as_ref().and_then(|p| p.id);
            }
            let res = drive
                .db_insert(pool)
                .await
                .map_err(|e| log::error!("Error inserting drive into database: {e}"))
                .map(|id| drive.id = id);

            if res.is_ok() {
                // Update drive_id of the position entry
                if let Some(ref p) = tables.position {
                    if let Err(e) = p.db_update_drive_id(pool, drive.id).await {
                        log::error!("Error updating position with drive_id: {e}");
                    }
                }
            }
        } else {
            // update the current drive
            drive.end_position_id = tables.position.as_ref().and_then(|p| p.id);
            if let Err(e) = drive.db_update(pool).await {
                log::error!("Error updating drive (id: {}): {e}", drive.id);
            }
        }
    }

    Ok(tables)
}

/// Get a map of VINs to car IDs from the database
// Read the list of cars from the database, we will check which car the vehicle_data response from the API belongs to
// It is more efficient to store the list of cars in memory and check against it instead of querying the database for each vehicle_data response
pub async fn get_vin_id_map(pool: &sqlx::PgPool) -> HashMap<String, i16> {
    #[rustfmt::skip]
    let vin_id_map = if let Ok(cars) = Car::db_get(pool).await {
        cars
            .iter()
            .filter(|c| c.id > 0 && c.vin.is_some()) // Remove entries with invalid id or None vins
            .map(|c| (c.vin.clone().expect("VIN is None, this should never happen"), c.id)) // Get vin and id from Car struct
            .collect()
    } else {
        log::error!("Error getting cars from database");
        HashMap::new()
    };

    vin_id_map
}

pub async fn logging_process(
    pool: &sqlx::PgPool,
    mut vin_id_map: HashMap<String, i16>,
    mut tables: Tables,
    data: &VehicleData,
) -> (HashMap<String, i16>, Tables) {
    let Some(vin) = &data.vin else {
        log::warn!("VIN is None, skipping this entry");
        return (vin_id_map, tables);
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
                return (vin_id_map, tables);
            }
        };
        let Ok(car) = Car::from(data, car_settings_id).map_err(|e| log::error!("Error creating car: {e}")) else {
            return (vin_id_map, tables);
        };
        let Ok(id) = car.db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}")).map(|id| id as i16)
        else {
            return (vin_id_map, tables);
        };
        vin_id_map.insert(vin.clone(), id);
        id
    };

    if let Ok(table_list) = create_tables_new(data, &tables, car_id)
        .await
        .map_err(|e| log::error!("Error adding to database: {e}"))
    {
        for t in table_list {
            match db_insert(pool, t).await {
                Ok(updated_tables) => tables = updated_tables,
                Err(e) => log::error!("Error inserting tables into database: {e}"),
            }
        }
    };

    (vin_id_map, tables)
}
