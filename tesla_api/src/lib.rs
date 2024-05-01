use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub mod auth;
pub mod response_codes;
pub mod stream;
pub mod utils;
pub mod vehicle_data;

pub use response_codes::TeslaResponseCode;

const BASE_URL: &str = "https://owner-api.teslamotors.com/api/1";
const AUTH_URL: &str = "https://auth.tesla.com/oauth2/v3/token";
const STREAMING_URL: &str = "wss://streaming.vn.teslamotors.com/streaming/";

pub type TeslaClient = reqwest::Client;

fn get_base_url() -> String {
    std::env::var("MOCK_TESLA_BASE_URL").unwrap_or_else(|_| BASE_URL.to_string())
}

#[derive(thiserror::Error, Debug)]
pub enum TeslaError {
    #[error("Connection Error: {0}")]
    Connection(#[from] reqwest::Error),
    #[error("Unexpected response `{0}`")]
    Request(StatusCode),
    #[error("{0}")]
    ApiError(TeslaResponseCode),
    #[error("Invalid response received from Tesla server: {0}")]
    InvalidResponse(String),
    #[error("Vehicle is not online")]
    NotOnline,
    #[error("API request timeout")]
    RequestTimeout,
    #[error("Invalid header value `{0}`")]
    InvalidHeader(reqwest::header::InvalidHeaderValue),
    #[error("Url parse error `{0}`")]
    ParseError(url::ParseError),
    #[error("WebSocket error `{0}`")]
    WebSocketError(tungstenite::Error),
    #[error("Access token expired, {0}")]
    TokenExpired(String),
    #[error("Error decoding json, {0}")]
    JsonDecodeError(serde_json::Error),
    #[error("Chipmunk code test in progress")]
    TestInProgress,
}

impl From<url::ParseError> for TeslaError {
    fn from(e: url::ParseError) -> TeslaError {
        TeslaError::ParseError(e)
    }
}

impl From<tungstenite::Error> for TeslaError {
    fn from(e: tungstenite::Error) -> TeslaError {
        TeslaError::WebSocketError(e)
    }
}

impl From<serde_json::Error> for TeslaError {
    fn from(e: serde_json::Error) -> TeslaError {
        TeslaError::JsonDecodeError(e)
    }
}

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    response: Option<T>,
    error: Option<String>,
    error_description: Option<String>,
    messages: Option<serde_json::Value>, // format -> {"field1":["problem1","problem2"],...}
}

impl<T: std::fmt::Display + std::fmt::Debug> std::fmt::Display for ApiResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "Response: {:?}, Error: {}, Error Description: {}, Message: {}",
                self.response,
                self.error.as_ref().unwrap_or(&"".into()),
                self.error_description.as_ref().unwrap_or(&"".into()),
                self.messages.as_ref().unwrap_or(&"".into())
            )
            .as_str(),
        )
    }
}

/// Macro to read and parse the JSON response from an HTTP request and convert it into a struct.
///
/// # Arguments
///
/// * `$response` - The HTTP response object.
/// * `$generic` - The type of the expected JSON response.
///
/// # Returns
///
/// Returns a Result containing the parsed JSON response if the HTTP status code is OK.
/// Otherwise, it returns an error indicating the reason for failure.
#[macro_export]
macro_rules! read_response_json {
    ($response:expr, $generic:ty) => {
        match $response.status() {
            StatusCode::OK => {
                let text = $response.text().await?;
                if text == "chipmunk_test_in_progress" {
                    return Err(TeslaError::TestInProgress);
                }

                match serde_json::from_str::<ApiResponse<$generic>>(&text)?.response {
                    Some(resp) => Ok(resp),
                    None => Err(TeslaError::InvalidResponse(text)),
                }
            }
            _ => parse_error!($response),
        }
    };
}

macro_rules! parse_error {
    ($response:expr) => {
        {
            let code = $response.status();

            let resp_text = $response.text().await?;
            // Try converting the response to a JSON object
            let resp = match serde_json::from_str::<ApiResponse<serde_json::Value>>(&resp_text) {
                Ok(json) => json.to_string(), // If successful, return the string representation of the JSON object
                Err(_) => resp_text, // Otherwise, return the raw response text
            };
            log::error!("{resp}");

            match code {
                // Check if the status code is a custom Tesla response code
                status_code => if let Ok(response_code) = TeslaResponseCode::from_http_status(status_code)
                {
                    match response_code {
                        TeslaResponseCode::UNAUTHORIZED => return Err(TeslaError::TokenExpired(format!(
                                "Status code `{}` received",
                                TeslaResponseCode::UNAUTHORIZED
                            ))),
                        TeslaResponseCode::DEVICE_NOT_AVAILABLE => Err(TeslaError::NotOnline), // Vehicle is not online
                        other_code => Err(TeslaError::ApiError(other_code))
                    }
                } else {
                    // Status code is not a custom Tesla response code (unknown code), return error
                    match status_code {
                        StatusCode::REQUEST_TIMEOUT => Err(TeslaError::RequestTimeout),
                        _ => Err(TeslaError::Request(status_code))
                    }
                }
            }
        }
    };
}

pub fn get_tesla_client(access_token: &str) -> Result<reqwest::Client, TeslaError> {
    let mut headers = reqwest::header::HeaderMap::new();
    let key = format!("Bearer {}", access_token);
    let mut auth_value = match reqwest::header::HeaderValue::from_str(&key) {
        Ok(value) => value,
        Err(e) => return Err(TeslaError::InvalidHeader(e)),
    };
    auth_value.set_sensitive(true);
    headers.insert(reqwest::header::AUTHORIZATION, auth_value);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok(client)
}

// TODO: merge this with the VehicleData struct in vehicle_data.rs
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Vehicles {
    pub id: Option<u64>,
    pub vehicle_id: Option<u64>,
    pub vin: Option<String>,
    pub display_name: Option<String>,
    pub option_codes: Option<String>,
    pub color: Option<String>,
    pub tokens: Option<Vec<String>>,
    pub state: Option<String>,
    pub in_service: Option<bool>,
    pub id_s: Option<String>,
    pub calendar_enabled: Option<bool>,
    pub api_version: Option<i32>,
    pub backseat_token: Option<String>,
    pub backseat_token_updated_at: Option<i32>,
}

pub async fn get_vehicles(client: &reqwest::Client) -> Result<Vec<Vehicles>, TeslaError> {
    log::debug!("Getting list of vehicles");
    let res = client.get(format!("{}/products", get_base_url())).send().await?;
    log::debug!("Received response: {:?}", res);
    read_response_json!(res, Vec<Vehicles>)
}

/*
 * id: value of `get_vehicles().id` field and not the `vehicle_id` field
*/
pub async fn get_vehicle_data(client: &reqwest::Client, id: u64) -> Result<String, TeslaError> {
    log::debug!("Getting vehicle data");
    let res = client
        .get(format!("{}/vehicles/{id}/vehicle_data", get_base_url()))
        .send()
        .await?;

    log::debug!("Received response: {:?}", res);
    Ok(read_response_json!(res, serde_json::Value)?.to_string())
}

pub struct Vehicle;
impl Vehicle {
    pub fn get_model_code(model_name: &Option<String>) -> Option<String> {
        let Some(name) = model_name else {
            log::warn!("model_name is `None`");
            return None;
        };

        let model_code = match name.to_lowercase().as_str() {
            "models" | "lychee" => "S",
            "model3" => "3",
            "modelx" | "tamarind" => "X",
            "modely" => "Y",
            s => {
                log::warn!("Unknown model name `{s}`");
                return None;
            }
        };

        Some(model_code.to_string())
    }

    pub fn get_marketing_name(
        model: Option<String>,
        trim_badging: Option<String>,
        m_type: Option<String>,
    ) -> Option<String> {
        let Some(model) = model else {
            log::warn!("Model is `None`");
            return None;
        };

        let Some(trim_badging) = trim_badging else {
            // log::warn!("trim_badging is `None`"); // TODO: uncomment this
            return None;
        };

        let Some(m_type) = m_type else {
            log::warn!("Model type is `None`");
            return None;
        };

        let model = model.to_ascii_uppercase();
        let trim_badging = trim_badging.to_ascii_uppercase();
        let m_type = m_type.to_ascii_lowercase();

        let marketing_name = match (model.as_str(), trim_badging.as_str(), m_type.as_str()) {
            ("S", "100D", "lychee") => "LR",
            ("S", "P100D", "lychee") => "Plaid",
            ("3", "P74D", _) => "LR AWD Performance",
            ("3", "74D", _) => "LR AWD",
            ("3", "74", _) => "LR",
            ("3", "62", _) => "MR",
            ("3", "50", _) => "SR+",
            ("X", "100D", "tamarind") => "LR",
            ("X", "P100D", "tamarind") => "Plaid",
            ("Y", "P74D", _) => "LR AWD Performance",
            ("Y", "74D", _) => "LR AWD",
            (m, tr, ty) => {
                log::warn!(
                    "Unknown combination of model `{m}`, trim_badging `{tr}`, and type `{ty}`"
                );
                return None;
            }
        };

        Some(marketing_name.to_string())
    }
}