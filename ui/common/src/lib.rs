mod status;

use chrono::{DateTime, Utc};
use macros::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use status::Status;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub enum Topic {
    #[serde(rename = "start")]
    StartLogging,
    #[serde(rename = "stop")]
    StopLogging,
    #[serde(rename = "get-server-settings")]
    GetServerSettings,
    #[serde(rename = "set-settings")]
    SetSettings,
    #[serde(rename = "get-settings")]
    GetSettings,
    #[serde(rename = "refresh-token")]
    RefreshToken,
    #[serde(rename = "logging-status")]
    LoggingStatus,
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub enum MessageType {
    #[default]
    #[serde(rename = "command")]
    Command,
    #[serde(rename = "response")]
    Response,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone, Json)]
pub struct WsMessage {
    pub id: String,
    pub r#type: MessageType,
    pub topic: Topic,
    pub data: Option<serde_json::Value>,
}

impl WsMessage {
    pub fn command(topic: Topic, data: Option<serde_json::Value>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            r#type: MessageType::Command,
            topic,
            data,
        }
    }

    pub fn response_with_data(&self, data: serde_json::Value) -> Self {
        Self {
            data: Some(data),
            r#type: MessageType::Response,
            ..self.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Json)]
pub struct WsMessageToken {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Json)]
pub struct LoggingStatus {
    pub timestamp: DateTime<Utc>,
    pub is_logging: bool,
    pub current_points: i32,
    pub total_points: i32,
    pub current_miles: i32,
    pub total_miles: i32,
    // pub session_start_time: DateTime<Utc>,
    // pub app_start_time: DateTime<Utc>,
    pub is_user_present: bool,
    pub odometer: i32,
    // Remove?
    pub charging_status: String,
    // pub Vehicle states
    // pub drive_state: DriveState,
    pub charge_state: String,
}
