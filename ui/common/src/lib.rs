use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// Trait to make JSON <-> Struct conversions easier
pub trait Json {
    fn to_string(&self) -> anyhow::Result<String>
    where
        Self: Serialize,
    {
        serde_json::to_string(&self).map_err(anyhow::Error::msg)
    }

    fn from_string<'a>(str: impl Into<&'a str>) -> anyhow::Result<Self>
    where
        Self: Sized,
        for<'b> Self: Deserialize<'b>,
    {
        serde_json::from_str(str.into()).map_err(anyhow::Error::msg)
    }

    fn from_value(value: serde_json::Value) -> anyhow::Result<Self>
    where
        Self: Sized,
        for<'c> Self: Deserialize<'c>,
    {
        serde_json::from_value(value).map_err(anyhow::Error::msg)
    }

    fn to_value(&self) -> anyhow::Result<serde_json::Value>
    where
        Self: Sized,
        Self: Serialize,
    {
        serde_json::to_value(self).map_err(anyhow::Error::msg)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct WsMessage {
    pub id: String,
    pub r#type: MessageType,
    pub topic: Topic,
    pub data: Option<serde_json::Value>,
}

impl Json for WsMessage {}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessageToken {
    pub token: String,
}

impl Json for WsMessageToken {}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct LoggingStatus {
    pub is_logging: bool,
    pub current_points: i32,
    pub total_points: i32,
    pub current_miles: i32,
    pub total_miles: i32,
    // pub session_start_time: NaiveDateTime,
    // pub app_start_time: NaiveDateTime,
    pub is_user_present: bool,
    pub odometer: i32,
    // Remove?
    pub charging_status: String,
    // pub Vehicle states
    // pub drive_state: DriveState,
    pub charge_state: String,
}

impl Json for LoggingStatus {}
