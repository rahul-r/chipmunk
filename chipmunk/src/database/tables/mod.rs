use chrono::NaiveDateTime;
use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use self::{
    address::Address, car::Car, charges::Charges, charging_process::ChargingProcess, drive::Drive,
    position::Position, settings::Settings, state::State, swupdate::SoftwareUpdate,
};

use super::{types::DriveStatus, DBTable};

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
}

impl Tables {
    pub fn get_time(&self) -> Option<NaiveDateTime> {
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

    pub fn from_vehicle_data(data: &VehicleData, car_id: i16) -> Self {
        Self {
            address: None,
            car: None,
            charges: Charges::from(data, 0)
                .map_err(|e| log::error!("Error creating charges from vehicle data: {e}"))
                .ok(),
            charging_process: None,
            drive: None,
            position: Position::from(data, car_id, None)
                .map_err(|e| log::error!("Error creating position from vehicle data: {e}"))
                .ok(),
            settings: None,
            state: State::from(data, car_id)
                .map_err(|e| log::error!("Error creating state from vehicle data: {e}"))
                .ok(),
            sw_update: None,
        }
    }

    pub async fn db_insert(&self, pool: &sqlx::PgPool) -> sqlx::Result<Self> {
        let mut tables = self.clone();

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
            if p.id.is_none() || p.id == Some(0) {
                p.db_insert(pool).await.map(|id| p.id = Some(id as i32))?; // Update id field of current_position with the id returned from the database
            }
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
                charging_process.position_id =
                    tables.position.as_ref().and_then(|p| p.id).unwrap_or(0);
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

    async fn _db_get_last(pool: &PgPool) -> Self {
        Self {
            address: Address::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            car: Car::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            charges: Charges::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            charging_process: ChargingProcess::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            drive: Drive::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            position: Position::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            settings: Settings::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            state: State::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
            sw_update: SoftwareUpdate::db_get_last(pool)
                .await
                .map_err(|e| log::error!("{e}"))
                .ok(),
        }
    }
}
