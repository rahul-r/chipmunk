use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::get_config;
use crate::tasks::DataTypes;
use tesla_api::{TeslaClient, TeslaError};
use tokio_util::sync::CancellationToken;

pub async fn data_polling_task(
    data_tx: mpsc::Sender<DataTypes>,
    config: Config,
    cancellation_token: CancellationToken,
    mut tesla_client: TeslaClient,
    car_id: u64,
) {
    let name = "data_polling_task";
    let mut _num_data_points = 0;
    loop {
        if cancellation_token.is_cancelled() {
            break;
        }

        match get_config!(config.logging_enabled) {
            Ok(false) => {
                //tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
            Ok(true) => (),
            Err(e) => {
                log::error!("Error getting config value `logging_enabled`: {e}");
                //tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        }

        let Ok(logging_period_ms) = get_config!(config.logging_period_ms) else {
            log::error!("Error reading config value `logging_period_ms`");
            return;
        };

        match tesla_api::get_vehicle_data(&mut tesla_client, car_id).await {
            Ok(data) => {
                if let Err(e) = data_tx.send(DataTypes::VehicleData(data)).await {
                    // don't log error message if the channel was closed because of a cancellation request
                    if !cancellation_token.is_cancelled() {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
            }
            Err(e) => {
                match e {
                    TeslaError::Connection(e) => log::error!("Error: `{e}`"),
                    TeslaError::Request(e) => log::error!("Error: `{e}`"),
                    TeslaError::ApiError(e) => log::error!("Error: `{e}`"), // TODO: Error: `429 - Account or server is rate limited. This happens when too many requests are made by an account.
                    // • Check the 'Retry-After' request header (in seconds); to determine when to make the next request.`
                    TeslaError::NotOnline => {
                        // TODO: Is there a way to wait for the vehicle to come online?
                        log::info!("Vehicle is not online");
                    }
                    TeslaError::InvalidHeader(e) => log::error!("Error: `{e}`"),
                    TeslaError::ParseError(e) => log::error!("Error: `{e}`"),
                    TeslaError::WebSocketError(e) => log::error!("Error: `{e}`"),
                    TeslaError::TokenExpired(e) => log::error!("Error: `{e}`"),
                    TeslaError::JsonDecodeError(e) => log::error!("Error: `{e}`"),
                    TeslaError::RequestTimeout => log::info!("Timeout"),
                    TeslaError::InvalidResponse(ref msg) => log::error!("Error: `{e}` - {msg}"),
                    TeslaError::TestInProgress => log::info!("{e}"),
                    TeslaError::Retry(e) => log::info!("{e}"),
                }
                tokio::time::sleep(Duration::from_millis(logging_period_ms as u64)).await;
                continue;
            }
        };

        _num_data_points += 1;

        tokio::time::sleep(Duration::from_millis(logging_period_ms as u64)).await;
    }

    tracing::warn!("exiting {name}");
}
