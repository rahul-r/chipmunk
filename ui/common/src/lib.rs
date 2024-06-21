mod status;
pub mod units;

use macros::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use status::{
    Charging, ClimateState, Driving, Logging, Offline, Parked, Sleeping, State, Status, Vehicle,
};

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
    #[serde(rename = "set-unit")]
    SetUnit,
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
