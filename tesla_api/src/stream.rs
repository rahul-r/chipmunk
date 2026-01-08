use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tungstenite::{connect, stream::MaybeTlsStream, Message, WebSocket};
use url::Url;

use crate::{utils::timestamp_to_datetime, TeslaError, STREAMING_URL};

#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct StreamingData {
    pub timestamp: Option<DateTime<Utc>>,
    pub speed: Option<f32>,
    pub odometer: Option<u32>,
    pub soc: Option<f32>,
    pub elevation: Option<f32>,
    pub est_heading: Option<f32>,
    pub est_lat: Option<f64>,
    pub est_lng: Option<f64>,
    pub power: Option<f32>,
    pub shift_state: Option<String>,
    pub range: Option<i32>,
    pub est_range: Option<i32>,
    pub heading: Option<f32>,
}

impl StreamingData {
    fn from(csv: Option<String>) -> anyhow::Result<Self> {
        if csv.is_none() {
            log::warn!("Invalid streaming data");
            anyhow::bail!("Invalid streaming data");
        }

        let csv = csv.expect("Invalid streamig data");
        let parts: Vec<&str> = csv.split(',').collect();

        if parts.len() != 13 {
            log::debug!("{parts:?}");
            anyhow::bail!("Expected 13 datafields, received {}", parts.len());
        }

        let streaming_data = StreamingData {
            timestamp: timestamp_to_datetime(parts[0].parse::<u64>().ok()),
            speed: parts[1].parse::<f32>().ok(),
            odometer: parts[2].parse::<u32>().ok(),
            soc: parts[3].parse::<f32>().ok(),
            elevation: parts[4].parse::<f32>().ok(),
            est_heading: parts[5].parse::<f32>().ok(),
            est_lat: parts[6].parse::<f64>().ok(),
            est_lng: parts[7].parse::<f64>().ok(),
            power: parts[8].parse::<f32>().ok(),
            shift_state: if parts[9].is_empty() {
                None
            } else {
                Some(parts[9].to_string())
            },
            range: parts[10].parse::<i32>().ok(),
            est_range: parts[11].parse::<i32>().ok(),
            heading: parts[12].parse::<f32>().ok(),
        };

        Ok(streaming_data)
    }
}

#[derive(Debug)]
enum StreamError {
    UnknownError(String),
    OwnerApiError(String),
    TokensExpired(String),
}

#[derive(Debug)]
enum MessageType {
    Start,
    Data(Option<StreamingData>),
    Error(StreamError),
    VehicleDisconnected,
    VehicleError,
    Timeout,
    Unknown(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WebSocketResponse {
    msg_type: String,
    connection_timeout: Option<u32>,
    tag: Option<String>,
    value: Option<String>,
    error_type: Option<String>,
}

fn parse_client_error(msg: Option<String>) -> StreamError {
    if let Some(m) = msg {
        match m.split_once(':') {
            Some(("owner_api error", data)) => StreamError::OwnerApiError(data.to_string()),
            Some(("Can't validate token", data)) => StreamError::TokensExpired(data.to_string()),
            Some((_, _)) => StreamError::UnknownError(m),
            None => StreamError::UnknownError("Unknown".to_string()),
        }
    } else {
        StreamError::UnknownError("None".to_string())
    }
}

#[allow(dead_code)]
impl WebSocketResponse {
    fn parse(&self) -> MessageType {
        match self.msg_type.as_str() {
            "control:hello" => MessageType::Start,
            "data:update" => match StreamingData::from(self.value.clone()) {
                Ok(v) => MessageType::Data(Some(v)),
                Err(e) => {
                    log::error!("{e}");
                    log::debug!("{self:?}");
                    MessageType::Data(None)
                }
            },
            "data:error" => match &self.error_type {
                Some(e) if e == "vehicle_disconnected" => MessageType::VehicleDisconnected,
                Some(e) if e == "vehicle_error" => MessageType::VehicleError,
                Some(e) if e == "client_error" => {
                    MessageType::Error(parse_client_error(self.value.clone()))
                }
                Some(e) if e == "timeout" => MessageType::Timeout,
                Some(e) => {
                    log::warn!("Unknown error message received from WebSocket `{e}`");
                    log::debug!("WebSocket message `{self:?}`");
                    MessageType::Error(StreamError::UnknownError(e.clone()))
                }
                None => MessageType::Error(StreamError::UnknownError("Unknown".to_string())),
            },
            unknown => {
                log::warn!("Unknown WebSocket message type `{unknown}`");
                log::debug!("WebSocket message `{self:?}`");
                MessageType::Unknown(format!("{self:?}"))
            }
        }
    }
}

/*
 * vehicle_id: value of `get_vehicles().vehicle_id` field and not the `id` field
 */
pub async fn start(
    access_token: &str,
    vehicle_id: u64,
    data_tx: mpsc::Sender<StreamingData>,
    cancellation_token: tokio_util::sync::CancellationToken,
) -> Result<(), TeslaError> {
    let create_websocket =
        || -> Result<WebSocket<MaybeTlsStream<std::net::TcpStream>>, TeslaError> {
            let url = Url::parse(STREAMING_URL)?;
            let (socket, _response) = match connect(url) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Error connecting to Tesla streaming websocket: {e}");
                    return Err(TeslaError::WebSocketError(e));
                }
            };
            Ok(socket)
        };

    let init_streaming =
        |socket: &mut WebSocket<MaybeTlsStream<std::net::TcpStream>>| -> Result<(), TeslaError> {
            let subscrib_message_oauth = format!(
                "{{
                    \"msg_type\": \"data:subscribe_oauth\",
                    \"token\": \"{access_token}\",
                    \"value\": \"speed,odometer,soc,elevation,est_heading,est_lat,est_lng,power,shift_state,range,est_range,heading\",
                    \"tag\": \"{vehicle_id}\"
                }}"
            );

            if let Err(e) = socket.send(Message::Text(subscrib_message_oauth)) {
                log::error!("{e}");
                return Err(TeslaError::WebSocketError(e));
            }

            Ok(())
        };

    let mut socket = create_websocket()?;
    init_streaming(&mut socket)?;

    loop {
        if cancellation_token.is_cancelled() {
            break;
        }

        let msg: WebSocketResponse = match socket.read() {
            Ok(v) => {
                if v.is_close() {
                    log::warn!("WebSocket is closing");
                    socket.close(None)?;
                    socket = create_websocket()?;
                    continue;
                }
                serde_json::from_str(v.to_text()?)?
            }
            Err(e) => {
                log::error!("{e}");
                socket.close(None)?;
                socket = create_websocket()?;
                continue;
            }
        };

        match msg.parse() {
            MessageType::Start => log::info!("TODO: handle Streaming started"),
            MessageType::Data(data) => {
                if let Some(d) = data {
                    log::debug!("{d:?}");
                    if let Err(e) = data_tx.send(d).await {
                        log::error!("Error sending streaming data over mpsc: {e}");
                    };
                }
            }
            MessageType::Error(e) => match e {
                StreamError::UnknownError(e) => log::warn!("TODO: handle error {e}"),
                StreamError::OwnerApiError(e) => log::warn!("TODO: handle error {e}"),
                StreamError::TokensExpired(e) => {
                    log::info!("Tokens expired: {e}");
                    return Err(TeslaError::TokenExpired(e));
                }
            },
            MessageType::VehicleDisconnected => {
                log::info!("Vehicle disconnected, trying to re-connect");
                init_streaming(&mut socket)?;
                continue;
            }
            MessageType::VehicleError => log::warn!("TODO: handle vehicle error"),
            MessageType::Timeout => log::warn!("TODO: handle Streaming timeout"),
            MessageType::Unknown(msg) => {
                log::warn!("TODO: handle Unknown message from WebSocket: {msg}");
            }
        };
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Ok(())
}
