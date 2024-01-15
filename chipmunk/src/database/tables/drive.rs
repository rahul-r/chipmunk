use crate::utils::{
    avg_option, max_option, min_option, sub_option, time_diff, time_diff_minutes_i16,
};
use chrono::NaiveDateTime;
use tesla_api::vehicle_data::ShiftState;

use super::{address::insert_address, position::Position};

#[derive(Debug, Default, Clone)]
pub struct Drive {
    pub id: i32,
    pub status: DriveStatus, // This is used to track the current status of charging
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
    pub outside_temp_avg: Option<f32>,
    pub speed_max: Option<f32>,
    pub power_max: Option<f32>,
    pub power_min: Option<f32>,
    pub start_ideal_range_km: Option<f32>,
    pub end_ideal_range_km: Option<f32>,
    pub start_km: Option<f32>,
    pub end_km: Option<f32>,
    pub distance: Option<f32>,
    pub duration_min: Option<i16>,
    pub car_id: i16,
    pub inside_temp_avg: Option<f32>,
    pub start_address_id: Option<i32>,
    pub end_address_id: Option<i32>,
    pub start_rated_range_km: Option<f32>,
    pub end_rated_range_km: Option<f32>,
    pub start_position_id: Option<i32>,
    pub end_position_id: Option<i32>,
    pub start_geofence_id: Option<i32>,
    pub end_geofence_id: Option<i32>,
}

impl Drive {
    pub fn start(
        position: &Position,
        car_foreign_key: i16,
        start_address_id: Option<i32>,
        start_geofence_id: Option<i32>,
    ) -> Self {
        Self {
            status: DriveStatus::Driving,
            car_id: car_foreign_key,
            start_date: position.date.unwrap_or_else(|| {
                log::warn!("Position date is None, using current system time");
                chrono::Utc::now().naive_utc()
            }),
            start_ideal_range_km: position.ideal_battery_range_km,
            start_km: position.odometer,
            end_ideal_range_km: position.ideal_battery_range_km,
            start_address_id,
            start_rated_range_km: position.rated_battery_range_km,
            end_rated_range_km: position.rated_battery_range_km,
            start_position_id: position.id,
            end_position_id: position.id,
            start_geofence_id,
            inside_temp_avg: position.inside_temp,
            outside_temp_avg: position.outside_temp,
            speed_max: position.speed,
            power_max: position.power,
            power_min: position.power,
            end_km: position.odometer,
            distance: Some(0.0),
            duration_min: Some(0),
            ..Self::default()
        }
    }

    pub fn reset(&self) -> Self {
        Self {
            id: self.id,
            ..Self::default()
        }
    }

    pub fn stop(
        &self,
        position: &Position,
        end_address_id: Option<i32>,
        end_geofence_id: Option<i32>,
    ) -> Self {
        Self {
            status: DriveStatus::NotDriving,
            end_date: position.date,
            end_position_id: position.id,
            end_address_id,
            end_geofence_id,
            ..self.update(position)
        }
    }

    pub fn update(&self, position: &Position) -> Self {
        Self {
            inside_temp_avg: avg_option(self.inside_temp_avg, position.inside_temp),
            outside_temp_avg: avg_option(self.outside_temp_avg, position.outside_temp),
            speed_max: max_option(self.speed_max, position.speed),
            power_min: min_option(self.power_min, position.power),
            power_max: max_option(self.power_max, position.power),
            end_ideal_range_km: position.ideal_battery_range_km,
            end_km: position.odometer,
            distance: sub_option(position.odometer, self.start_km),
            duration_min: time_diff_minutes_i16(Some(self.start_date), position.date),
            end_rated_range_km: position.rated_battery_range_km,
            end_position_id: position.id,
            ..self.clone()
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default, sqlx::Type)]
#[sqlx(type_name = "drive_stat", rename_all = "snake_case")]
pub enum DriveStatus {
    Start,
    /// Start a new drive
    Driving,
    /// Currently driving, record the position/drive statistics data
    Stop,
    /// Stop the current drive
    NotDriving,
    /// Not driving, waiting for a drive to start
    Restart,
    /// Stop the current drive and immediately start a new one.
    /// Use the previous data point as the last data point for the previous drive
    /// and use the current data point as the starting of a new drive (Leave the end_date None to
    /// mark we don't know when the current drive ended).
    /// Leave the start_date None to mark we don't know when the drive was started.
    #[default]
    Unknown, // Unknown state, do nothing
}

/// Checks if a drive has restarted based on the previous and current positions.
///
/// # Arguments
///
/// * `previous` - An `Option` that holds a reference to the previous `Position`.
/// * `current` - A reference to the current `Position`.
///
/// # Returns
///
/// * `bool` - Returns `true` if the drive has restarted, else `false`.
///     Returns `false` if any of the following conditions are met:
///     * The `previous` position is `None`.
///     * The time difference between the `previous` and `current` positions is less than 10 minutes.
///     * The distance between the `previous` and `current` positions is more than 1 mile.
///     * There's an error.
fn drive_restarted(previous: Option<&Position>, current: &Position) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    let Some(diff) = time_diff(previous.date, current.date) else {
        return false;
    };
    if diff.num_minutes() < 10 {
        return false;
    }
    let Some(distance) = sub_option(current.odometer, previous.odometer) else {
        return false;
    };
    if distance > 1.0 {
        return false;
    }
    log::info!("Drive offline, stopping the current drive and starting a new one");
    false
}

pub fn drive_status(
    previous_position: Option<&Position>,
    current_position: &Position,
    current_shift: Option<ShiftState>,
    current_status: &DriveStatus,
) -> DriveStatus {
    if current_status == &DriveStatus::Driving {
        if drive_restarted(previous_position, current_position) {
            return DriveStatus::Restart;
        }
    }

    // If the shift state is Parked on None, the car is not driving, else tell the logger to start a new drive
    let is_parked = match current_shift {
        Some(ShiftState::P) | None => true,
        Some(_) => false,
    };

    // If the drive status is unknown, assume a state based on the current shift state
    if *current_status == DriveStatus::Unknown {
        let state = if is_parked {
            DriveStatus::Stop
        } else {
            DriveStatus::Start
        };
        log::info!("Unknown drive status, assuming `{:?}` based on the current shift state ({:?})", state, current_shift);
        return state;
    } else if *current_status == DriveStatus::Start {
        return DriveStatus::Driving;
    } else if *current_status == DriveStatus::Stop {
        return DriveStatus::NotDriving;
    }

    if *current_status == DriveStatus::Driving && is_parked {
        return DriveStatus::Stop;
    }
    if *current_status == DriveStatus::NotDriving && !is_parked {
        return DriveStatus::Start;
    }

    return if is_parked {
        DriveStatus::NotDriving
    } else {
        DriveStatus::Driving
    };
}

pub async fn start(pool: &sqlx::PgPool, start_position: &Position, car_id: i16) -> Drive {
    let start_address_id =
        insert_address(pool, start_position.latitude, start_position.longitude).await;

    let start_geofence_id = None; // TODO: Add this

    let mut new_drive =
        Drive::start(&start_position, car_id, start_address_id, start_geofence_id);

    match new_drive.db_insert(pool).await {
        Ok(id) => {
            // Update the drive_id of position entry
            if let Err(e) = start_position.db_update_drive_id(pool, id).await {
                log::error!("Error updating position with drive_id: {e}");
            }
            new_drive.id = id;
        }
        Err(e) => log::error!("Error inserting drive into database: {e}"),
    }

    new_drive
}

pub async fn stop(pool: &sqlx::PgPool, drive: Drive, stop_position: &Position) -> Drive {
    let end_addr_id =
        insert_address(pool, stop_position.latitude, stop_position.longitude).await;

    let end_geofence_id = None; // TODO: Add this

    let end_drive = drive.stop(&stop_position, end_addr_id, end_geofence_id);
    if let Err(e) = end_drive.db_update(pool).await {
        log::error!("Error marking drive (id: {}) as stopped: {e}", drive.id);
    } else {
    }

    end_drive.reset()
}

pub async fn update(pool: &sqlx::PgPool, drive: Drive, current_position: &Position) -> Drive {
    let updated_drive = drive.update(&current_position);
    if let Err(e) = updated_drive.db_update(pool).await {
        log::error!("Error updating drive (id: {}): {e}", drive.id);
    }

    // Update the drive_id of position entry
    if drive.id != 0 {
        if let Err(e) = current_position
            .db_update_drive_id(pool, updated_drive.id)
            .await
        {
            log::error!("Error updating position with drive_id: {e}");
        }
    }

    updated_drive
}

pub async fn handle_drive(
    pool: &sqlx::PgPool,
    previous_position: Option<Position>,
    current_position: &Position,
    current_shift: Option<ShiftState>,
    drive: Drive,
    car_id: i16,
) -> Drive {
    // TODO: Run this only when the vehicle state is changing from offline to online
    // if driven_offline(previous_position.as_ref(), current_position) {
    //     log::info!("Vehicle driven offline, creating and inserting a new drive into database");
    //     let drive_start_position = match previous_position {
    //         Some(p) => p,
    //         None => todo!(),
    //     };

    //     let mut drive = Drive::start(&drive_start_position, car_id, None, None)
    //         .update(current_position)
    //         .stop(current_position, None, None);

    //     match drive.db_insert(pool).await {
    //         Ok(id) => drive.id = id,
    //         Err(e) => log::error!("Error inserting drive to database: {e}"),
    //     }

    //     return drive.reset();
    // }

    let position_id = match current_position.db_insert(pool).await {
        Ok(id) => Some(id as i32),
        Err(e) => {
            log::error!("Error adding position to database: {e}");
            None
        }
    };

    let current_position = Position {
        id: position_id,
        ..current_position.clone()
    };

    let status = drive_status(
        previous_position.as_ref(),
        &current_position,
        current_shift,
        &drive.status,
    );

    let drive = match status {
        DriveStatus::Start => start(pool, &current_position, car_id).await,
        DriveStatus::Driving => {
            let updated_drive = drive.update(&current_position);
            if let Err(e) = updated_drive.db_update(pool).await {
                log::error!("Error updating drive (id: {}): {e}", drive.id);
            }

            // Update the drive_id of position entry
            if drive.id != 0 {
                if let Err(e) = current_position
                    .db_update_drive_id(pool, updated_drive.id)
                    .await
                {
                    log::error!("Error updating position with drive_id: {e}");
                }
            }

            updated_drive
        }
        DriveStatus::Stop => stop(pool, drive, &current_position).await,
        DriveStatus::Restart => {
            log::info!(
                "Restarting drive (Stopping drive id = {} and starting a new one)",
                drive.id
            );
            // If a drive is active, stop it
            if let Some(position) = previous_position {
                stop(pool, drive, &position).await;
            }
            start(pool, &current_position, car_id).await
        }
        DriveStatus::NotDriving => drive.reset(),
        DriveStatus::Unknown => {
            log::warn!("Unknown drive state");
            drive
        }
    };

    Drive { status, ..drive }
}

#[tokio::test]
#[cfg_attr(rustfmt, rustfmt_skip)]
pub async fn test_handle_drive() {
    dotenvy::dotenv().ok();
    let env = crate::environment::load().unwrap();
    let pool = crate::database::initialize(&env.database_url).await.unwrap();

    let test_shift_state = |pool: sqlx::PgPool, drive_status: DriveStatus, curr_state: Option<ShiftState>| async move {
        let previous_position = Some(Position::default());
        let current_position = Position::default();
        let drive = Drive {
            status: drive_status,
            ..Drive::default()
        };

        let drive = handle_drive(
            &pool,
            previous_position,
            &current_position,
            curr_state,
            drive,
            1,
        ).await;
        return drive.status;
    };

    assert_eq!(test_shift_state(pool.clone(), DriveStatus::NotDriving, Some(ShiftState::P)).await, DriveStatus::NotDriving);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::NotDriving, Some(ShiftState::N)).await, DriveStatus::Start);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::NotDriving, Some(ShiftState::D)).await, DriveStatus::Start);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::NotDriving, Some(ShiftState::R)).await, DriveStatus::Start);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::NotDriving, None).await, DriveStatus::NotDriving);

    assert_eq!(test_shift_state(pool.clone(), DriveStatus::Driving, Some(ShiftState::P)).await, DriveStatus::Stop);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::Driving, Some(ShiftState::N)).await, DriveStatus::Driving);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::Driving, Some(ShiftState::D)).await, DriveStatus::Driving);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::Driving, Some(ShiftState::R)).await, DriveStatus::Driving);
    assert_eq!(test_shift_state(pool.clone(), DriveStatus::Driving, None).await, DriveStatus::Stop);
}
