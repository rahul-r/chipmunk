use std::collections::HashMap;

use chrono::{Duration, NaiveDateTime, Utc};
use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use crate::{
    database::tables::car::CarSettings,
    utils::{sub_option, time_diff},
};

use self::{
    address::Address,
    car::Car,
    charging::{Charges, ChargingProcess},
    drive::{Drive, DriveStatus},
    position::Position,
    settings::Settings,
    state::State,
    swupdate::SoftwareUpdate,
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

pub async fn initialize(pool: &PgPool) -> anyhow::Result<()> {
    settings::initialize(pool).await?;
    Ok(())
}

pub trait DBTable {
    // equired methods
    fn table_name() -> &'static str;
    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64>;

    // Optional methods
    async fn db_update(&self, _pool: &PgPool) -> sqlx::Result<()> {
        #[rustfmt::skip]
        panic!("{}", format!("`db_update` is not implemented for `{}` table!", Self::table_name()))
    }
    async fn db_update_last(&self, _pool: &PgPool) -> sqlx::Result<()>
    where
        Self: Sized,
    {
        #[rustfmt::skip]
        panic!("{}", format!("`db_update_last` is not implemented for `{}` table!", Self::table_name()))
    }
    async fn db_get_last(_pool: &PgPool) -> sqlx::Result<Self>
    where
        Self: Sized,
    {
        #[rustfmt::skip]
        panic!("{}", format!("`db_get_last` is not implemented for `{}` table!", Self::table_name()))
    }
    async fn db_get_all(_pool: &PgPool) -> sqlx::Result<Vec<Self>>
    where
        Self: Sized,
    {
        #[rustfmt::skip]
        panic!("{}", format!("`db_get_all` is not implemented for `{}` table!", Self::table_name()))
    }
    async fn db_num_rows(pool: &PgPool) -> sqlx::Result<i64> {
        let resp = sqlx::query(
            format!(r#"SELECT COUNT(*) as count FROM {}"#, Self::table_name()).as_str(),
        )
        .fetch_one(pool)
        .await?;
        Ok(sqlx::Row::get::<i64, _>(&resp, "count"))
    }
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
    data: &VehicleData,
    prev_tables: &Tables,
    car_id: i16,
) -> anyhow::Result<Vec<Tables>> {
    let current_state = State::from(data, car_id)?;
    let current_position = Position::from(data, car_id, None)?;
    let mut drive = prev_tables.drive.clone();
    let mut charging_process = prev_tables.charging_process.clone();
    let mut charges = Charges::from(data, 0).ok();
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
                    ..previous_state.clone().unwrap_or_default()
                };
                table_list.push(Tables {
                    address: address.clone(),
                    drive: drive.map(|d| d.stop(&current_position, None, None)),
                    position: Some(current_position.clone()),
                    state: Some(state),
                    ..Default::default()
                });

                // Check if the vehicle was charged since the previous data point
                let prev_battery_level =
                    prev_tables.position.as_ref().and_then(|p| p.battery_level);
                let curr_battery_level = current_position.battery_level;
                if let Some(charge) = sub_option(curr_battery_level, prev_battery_level) {
                    // Continue only if the battery level us up by at least 2% (>1)
                    if charge > 1 {
                        let current_charge = Charges::from(data, 0)?;
                        // Create a new charging process
                        ChargingProcess::from_charges(
                            prev_tables.charges.as_ref(),
                            &current_charge,
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
                                charges: Some(current_charge),
                                position: Some(current_position.clone()),
                                state: Some(State {
                                    state: state::StateStatus::Charging,
                                    start_date: prev_tables.time().unwrap_or_default(),
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

                // Start a new drive
                let new_drive = Drive {
                    start_position_id: prev_position.id,
                    ..Drive::start(&current_position, car_id, None, None)
                };
                let state = Some(State {
                    state: state::StateStatus::Driving,
                    start_date: current_position.date.unwrap_or_default(),
                    car_id,
                    ..Default::default()
                });
                table_list.push(Tables {
                    address,
                    drive: Some(new_drive),
                    position: Some(current_position),
                    state,
                    ..Default::default()
                });
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
                position = Some(Position {
                    drive_id: drive.as_ref().map(|d| d.id),
                    ..current_position.clone()
                });
            }
            Offline => todo!(),
            Asleep => todo!(),
            Unknown => todo!(),
            Parked => drive = None,
            Charging => {
                Charges::from(data, 0)
                    .map_err(|e| log::error!("Error creating charges: {e}"))
                    .map(|c| {
                        charges = Some(c.clone());
                        charging_process = charging_process.clone().map(|cp| cp.update(&c));
                    })
                    .ok();
            }
        }

        table_list.push(Tables {
            drive: drive.clone(),
            address: None, // Don't log address with every drive status update to reduce strain on openstreetmap API
            car: None,
            charges: charges.clone(),
            charging_process: charging_process.clone(),
            position,
            settings: None,
            state,
            sw_update: None,
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
            Charging => {
                Charges::from(data, 0)
                    .map_err(|e| log::error!("Error creating charges: {e}"))
                    .map(|c| {
                        charges = Some(c.clone());
                        charging_process = charging_process.clone().map(|cp| cp.update(&c));
                    })
                    .ok();
            }
            Asleep => todo!(),
            Offline => todo!(),
            Unknown => todo!(),
            Parked => position = None,
        }

        table_list.push(Tables {
            address: address.clone(),
            car: None,
            charges: charges.clone(),
            charging_process: charging_process.clone(),
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
            Charging => {
                let c = Charges::from(data, 0)
                    .map_err(|e| log::error!("Error creating charges: {e}"))
                    .ok();
                if let Some(c) = c {
                    let cp = ChargingProcess::start(
                        &c,
                        car_id,
                        current_position.id.unwrap_or(0),
                        None,
                        None,
                    );
                    charging_process = Some(cp);
                    charges = Some(c);
                    address =
                        Address::from_opt(current_position.latitude, current_position.longitude)
                            .await
                            .map_err(|e| log::error!("Error getting address: {e}"))
                            .ok();
                };
            }
            Asleep => todo!(),
            Offline => todo!(),
            Unknown => todo!(),
            Parked => {
                position = None;
                drive = None;
            }
        }

        table_list.push(Tables {
            address,
            car: None,
            charges,
            charging_process,
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

    // Insert state table
    if let Some(ref mut s) = tables.state {
        if s.id == 0 {
            s.db_insert(pool).await.map(|id| s.id = id as i32)?;
        } else {
            s.db_update(pool).await?;
        }
    }

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
                .map(|id| drive.id = id as i32);

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

    if let Some(ref mut charging_process) = tables.charging_process {
        if charging_process.id == 0 {
            charging_process.position_id = tables.position.as_ref().and_then(|p| p.id).unwrap_or(0);
            charging_process.address_id = address_id;

            charging_process
                .db_insert(pool)
                .await
                .map(|id| charging_process.id = id as i32)?;
        } else {
            charging_process.db_update(pool).await?;
        }

        if let Some(ref mut charges) = tables.charges {
            charges.charging_process_id = charging_process.id;
            charges
                .db_insert(pool)
                .await
                .map(|id| charges.id = id as i32)?;
        }
    }

    Ok(tables)
}

/// Get a map of VINs to car IDs from the database
// Read the list of cars from the database, we will check which car the vehicle_data response from the API belongs to
// It is more efficient to store the list of cars in memory and check against it instead of querying the database for each vehicle_data response
pub async fn get_vin_id_map(pool: &sqlx::PgPool) -> HashMap<String, i16> {
    #[rustfmt::skip]
    let vin_id_map = if let Ok(cars) = Car::db_get_all(pool).await {
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
        // TODO: move to the main db_insert function
        let Ok(id) = car.db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}")).map(|id| id as i16)
        else {
            return (vin_id_map, tables);
        };
        vin_id_map.insert(vin.clone(), id);
        id
    };

    if let Ok(table_list) = create_tables(data, &tables, car_id)
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
