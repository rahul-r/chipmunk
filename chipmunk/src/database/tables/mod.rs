use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use crate::DELAYED_DATAPOINT_TIME_SEC;

use self::{
    address::Address,
    car::Car,
    charges::Charges,
    charging_process::ChargingProcess,
    drive::Drive,
    position::Position,
    settings::Settings,
    state::{State, StateStatus},
    swupdate::SoftwareUpdate,
};

use super::DBTable;

pub mod address;
pub mod car;
pub mod car_settings;
pub mod charges;
pub mod charging_process;
pub mod drive;
pub mod geofence;
pub mod position;
pub mod settings;
pub mod state;
pub mod swupdate;
pub mod token;
pub mod vehicle_data;

pub async fn initialize(pool: &PgPool) -> anyhow::Result<()> {
    settings::initialize(pool).await?;
    Ok(())
}

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
    pub time: Option<DateTime<Utc>>,
    pub raw_data: Option<VehicleData>,
}

impl Tables {
    pub fn get_time(&self) -> Option<DateTime<Utc>> {
        self.time
    }

    fn is_state(&self, state: StateStatus) -> bool {
        self.state
            .as_ref()
            .map(|s| s.state == state)
            .unwrap_or(false)
    }

    pub fn is_driving(&self) -> bool {
        self.is_state(StateStatus::Driving)
    }

    pub fn is_charging(&self) -> bool {
        self.is_state(StateStatus::Charging)
    }

    pub fn car_id(&self) -> i16 {
        self.position.as_ref().map(|p| p.car_id).unwrap_or_else(|| {
            log::error!("No car_id found in position table");
            0
        })
    }

    // pub async fn from_vehicle_data(data: &VehicleData, car_id: i16) -> Self {
    //     Self {
    //         address: None,
    //         car: None,
    //         charges: Charges::from(data, 0)
    //             .map_err(|e| log::error!("Error creating charges from vehicle data: {e}"))
    //             .ok(),
    //         charging_process: None,
    //         drive: None,
    //         position: Position::from(data, car_id, None)
    //             .await
    //             .map_err(|e| log::error!("Error creating position from vehicle data: {e}"))
    //             .ok(),
    //         settings: None,
    //         state: State::from(data, car_id)
    //             .map_err(|e| log::error!("Error creating state from vehicle data: {e}"))
    //             .ok(),
    //         sw_update: None,
    //         time: data.timestamp_utc(),
    //         raw_data: Some(data.clone()),
    //     }
    // }

    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<Self> {
        let mut tables = self.clone();

        // Insert state table
        if let Some(ref mut s) = tables.state {
            if s.id == 0 {
                s.db_insert(pool)
                    .await
                    .map(|id| s.id = id as i32)
                    .map_err(|e| log::error!("{:?}", e))
                    .ok();
            } else {
                s.db_update(pool)
                    .await
                    .map_err(|e| log::error!("{:?}", e))
                    .ok();
            }
        }

        // Insert position and update the ID field
        if let Some(ref mut p) = tables.position {
            if p.id.is_none() || p.id == Some(0) {
                p.db_insert(pool).await.map(|id| p.id = Some(id as i32))?; // Update id field of current_position with the id returned from the database
            }
        }

        // Insert address and update the ID field
        let address_id = if let Some(ref mut address) = tables.address {
            if address.id != 0 {
                // If address id is not 0, address is already in the database, jsut return the id
                Some(address.id as i32)
            } else {
                address
                    .db_insert(pool)
                    .await
                    .map_err(|e| log::error!("Error inserting address into database: {e}"))
                    .map(|id| {
                        address.id = id;
                        id as i32
                    })
                    .ok()
            }
        } else {
            None
        };

        if let Some(ref mut drive) = tables.drive {
            if address_id.is_some() {
                if drive.in_progress {
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
                charging_process.position_id =
                    tables.position.as_ref().and_then(|p| p.id).unwrap_or(0);
                charging_process.address_id = address_id;

                charging_process
                    .db_insert(pool)
                    .await
                    .map(|id| charging_process.id = id as i32)?;
            }
        }

        // Insert charges and update the charging process
        if let Some(ref mut charges) = tables.charges {
            charges
                .db_insert_for_last_charging_process(pool)
                .await
                .map(|id| charges.id = id as i32)?;
            ChargingProcess::db_recalculate(pool, tables.charges.as_ref()).await?;
            Car::update_efficiency(pool, tables.car_id()).await?;
        }

        Ok(tables)
    }

    pub async fn db_get_last(pool: &PgPool) -> Self {
        let position = Position::db_get_last(pool)
            .await
            .map_err(|e| log::warn!("Position: {e}"))
            .ok();

        let time = position.as_ref().and_then(|p| p.date);

        let time_now = chrono::offset::Utc::now();

        let mut state = State::db_get_last(pool)
            .await
            .map_err(|e| log::warn!("State: {e}"))
            .ok();

        // Get the last charging process data point if it was logged less than DELAYED_DATAPOINT_TIME_SEC seconds ago
        // If it was logged more than DELAYED_DATAPOINT_TIME_SEC seconds ago, return None to create a new charging process
        let charging_process = ChargingProcess::db_get_last(pool)
            .await
            .map_err(|e| log::warn!("ChargingProcess: {e}"))
            .map(|cp| {
                cp.end_date
                    .map(|end_time| time_now - end_time)
                    .map(|diff| diff.num_seconds() <= DELAYED_DATAPOINT_TIME_SEC)
                    .inspect(|continue_charging| if !continue_charging {
                        log::debug!("Last charging process data point was logged more than {DELAYED_DATAPOINT_TIME_SEC} seconds ago. Returning None to creare a new charging process");
                    })
                    .map(|continue_charging| if continue_charging { Some(cp) } else { state = None; None })
                    .unwrap_or(None)
            })
            .unwrap_or(None);

        // Get the last drive data point if it was logged less than DELAYED_DATAPOINT_TIME_SEC seconds ago
        // If it was logged more than DELAYED_DATAPOINT_TIME_SEC seconds ago, return None to create a new drive
        let drive = Drive::db_get_last(pool)
            .await
            .map_err(|e| log::warn!("Drive: {e}"))
            .map(|drv| {
                drv.end_date
                    .map(|end_time| time_now - end_time)
                    .map(|diff| diff.num_seconds() <= DELAYED_DATAPOINT_TIME_SEC)
                    .inspect(|continue_drive| if !continue_drive {
                        log::debug!("Last drive data point was logged more than {DELAYED_DATAPOINT_TIME_SEC} seconds ago. Returning None to creare a new idrive");
                    })
                    .map(|continue_drive| if continue_drive { Some(drv) } else { state = None; None })
                    .unwrap_or(None)
            })
            .unwrap_or(None);

        Self {
            address: Address::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("Address: {e}"))
                .ok(),
            car: Car::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("Car: {e}"))
                .ok(),
            charges: Charges::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("Charges: {e}"))
                .ok(),
            charging_process,
            drive,
            position,
            settings: Settings::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("Settings: {e}"))
                .ok(),
            state,
            sw_update: SoftwareUpdate::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("SoftwareUpdate: {e}"))
                .ok(),
            time,
            raw_data: VehicleData::db_get_last(pool)
                .await
                .map_err(|e| log::warn!("VehicleData: {e}"))
                .ok(),
        }
    }
}
