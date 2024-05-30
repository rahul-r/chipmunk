use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

use macros::Json;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
enum State {
    Driving,
    Charging,
    Sleeping,
    Parked,
    Offline,
    #[default]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
struct Driving {
    location: String,
    start_time: DateTime<Utc>,
    duration_sec: u32,
    miles_driven: u32,
    charge_used: f32,
    destination: String,
    time_remaining_sec: u32,
    charge_at_destination: f32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
struct Charging {
    location: String,
    start_time: DateTime<Utc>,
    duration_sec: u32,
    charge_added: u32,
    cost: String,
    time_remaining_sec: u32,
    interior_temperature: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
struct Parked {
    location: String,
    start_time: DateTime<Utc>,
    duration_sec: u32,
    charge: u32,
    charge_used: u32,
    interior_temperature: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
struct Offline {
    start_time: DateTime<Utc>,
    duration_sec: u32,
    last_known_location: String,
    last_known_charge: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
struct Sleeping {
    start_time: DateTime<Utc>,
    duration_sec: u32,
    location: String,
    last_known_charge: u32,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct Status {
    state: State,
    driving: Option<Driving>,
    charging: Option<Charging>,
    parked: Option<Parked>,
    offline: Option<Offline>,
    sleeping: Option<Sleeping>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct LoggingStatus {
    pub timestamp: DateTime<Utc>,
    pub is_logging: bool,
    pub current_points: i32,
    pub total_points: i32,
    pub current_miles: i32,
    pub total_miles: i32,
    // pub app_start_time: DateTime<Utc>,
    pub is_user_present: bool,
    pub odometer: i32,
    // Remove?
}
