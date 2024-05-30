use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use macros::Json;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub enum State {
    Driving,
    Charging,
    Sleeping,
    Parked,
    Offline,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Driving {
    pub location: String,
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub miles_driven: u32,
    pub charge_used: f32,
    pub destination: String,
    pub time_remaining_sec: u32,
    pub charge_at_destination: f32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Charging {
    pub location: String,
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub charge_added: u32,
    pub cost: String,
    pub time_remaining_sec: u32,
    pub interior_temperature: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Parked {
    pub location: String,
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub charge: u32,
    pub charge_used: u32,
    pub interior_temperature: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Offline {
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub last_known_location: String,
    pub last_known_charge: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Sleeping {
    pub start_time: DateTime<Utc>,
    pub duration_sec: u32,
    pub location: String,
    pub last_known_charge: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Vehicle {
    pub odometer: i32,
    pub is_user_nearby: bool,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct LoggingStatus {
    pub enabled: bool,
    pub current_num_points: i32,
    pub total_num_points: i32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Status {
    pub timestamp: DateTime<Utc>,
    pub app_start_time: DateTime<Utc>,
    pub state: State,
    pub logging: LoggingStatus,
    pub vehicle: Vehicle,
    pub driving: Option<Driving>,
    pub charging: Option<Charging>,
    pub parked: Option<Parked>,
    pub offline: Option<Offline>,
    pub sleeping: Option<Sleeping>,
}
