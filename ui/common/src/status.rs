use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use macros::Json;

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub enum State {
    Driving,
    Charging,
    Sleeping,
    Parked,
    Offline,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Eq, PartialEq)]
pub enum ClimateState {
    Off,
    AC,
    Heater,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Driving {
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub miles_driven: u32,
    pub starting_battery_level: Option<i16>,
    pub current_battery_level: Option<i16>,
    pub charge_used: i16,
    pub battery_level_at_destination: f32,
    pub destination: Option<String>,
    pub time_remaining_sec: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Charging {
    pub start_time: DateTime<Utc>,
    pub duration_sec: i64,
    pub starting_battery_level: Option<i16>,
    pub current_battery_level: Option<i16>,
    pub charge_added: f32,
    pub cost: u32,
    pub time_remaining_sec: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Parked {
    pub start_time: DateTime<Utc>,
    pub duration_sec: i64,
    pub starting_battery_level: Option<i16>,
    pub current_battery_level: Option<i16>,
    pub charge_used: i16,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Offline {
    pub start_time: DateTime<Utc>,
    pub duration_sec: i64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Sleeping {
    pub start_time: DateTime<Utc>,
    pub duration_sec: i64,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Vehicle {
    pub name: String,
    pub odometer: f32,
    pub is_user_nearby: bool,
    pub is_locked: Option<bool>,
    pub location: Option<String>,
    pub battery_level: Option<i16>,
    pub range: Option<f32>,
    pub interior_temperature: Option<f32>,
    pub exterior_temperature: Option<f32>,
    pub climate_control_state: Option<ClimateState>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Logging {
    pub enabled: bool,
    pub current_num_points: i32,
    pub total_num_points: i32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Status {
    pub timestamp: DateTime<Utc>,
    pub app_start_time: DateTime<Utc>,
    pub state: State,
    pub logging: Logging,
    pub vehicle: Vehicle,
    pub driving: Option<Driving>,
    pub charging: Option<Charging>,
    pub parked: Option<Parked>,
    pub offline: Option<Offline>,
    pub sleeping: Option<Sleeping>,
}
