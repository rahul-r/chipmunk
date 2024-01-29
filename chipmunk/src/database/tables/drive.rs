use crate::utils::{
    avg_option, max_option, min_option, sub_option, time_diff_minutes_i16,
};
use chrono::NaiveDateTime;

use super::position::Position;

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
