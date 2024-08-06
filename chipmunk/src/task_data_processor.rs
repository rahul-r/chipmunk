use crate::config::Config;
use crate::database;
use crate::database::tables::Tables;
use crate::logger::{create_tables, get_car_id};
use crate::tasks::{DataTypes, DatabaseDataType, DatabaseRespType};
use tesla_api::vehicle_data::VehicleData;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

pub async fn data_processor_task(
    mut vehicle_data_rx: mpsc::Receiver<DataTypes>,
    processed_data_tx: broadcast::Sender<Tables>,
    database_tx: mpsc::Sender<DatabaseDataType>,
    mut database_resp_rx: mpsc::Receiver<DatabaseRespType>,
    _config: Config,
    cancellation_token: CancellationToken,
    pool: &sqlx::PgPool,
) {
    use mpsc::error::*;
    let name = "data_processor_task";
    let mut vin_id_map = database::tables::car::get_vin_id_map(pool).await;
    let mut prev_tables = Tables::db_get_last(pool).await;

    loop {
        tokio::task::yield_now().await;

        match vehicle_data_rx.try_recv() {
            Ok(v) => match v {
                DataTypes::VehicleData(data) => {
                    if let Err(e) = database_tx
                        .send(DatabaseDataType::RawData(data.clone()))
                        .await
                    {
                        log::error!("{name}: cannot send raw vehicle data over database_tx: {e}");
                    }

                    let vehicle_data = match VehicleData::from_response_json(&data) {
                        Ok(data) => data,
                        Err(e) => {
                            log::error!("Error parsing vehicle data to json: {e}");
                            continue;
                        }
                    };

                    let car_id_opt;
                    (vin_id_map, car_id_opt) = get_car_id(pool, vin_id_map, &vehicle_data).await;

                    let Some(car_id) = car_id_opt else {
                        log::error!("Error getting car ID");
                        continue;
                    };

                    let table_list = match create_tables(&vehicle_data, &prev_tables, car_id).await
                    {
                        Ok(table_list) => table_list,
                        Err(e) => {
                            log::error!("Error adding to database: {e}");
                            continue;
                        }
                    };

                    // Send the tables to the database task
                    if let Err(e) = database_tx.send(DatabaseDataType::Tables(table_list)).await {
                        log::error!("{name}: cannot send table_list over database_tx: {e}");
                    }

                    // Wait for the response from database task with the updated tables with
                    // database id fields
                    if let Some(resp) = database_resp_rx.recv().await {
                        if let DatabaseRespType::Tables(prev_tables_resp) = resp {
                            prev_tables = prev_tables_resp;
                        } else {
                            log::error!("Unexpected response type received from database task");
                        }
                    } else {
                        log::error!("No response received from database task");
                    }

                    if let Err(e) = processed_data_tx.send(prev_tables.clone()) {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
                DataTypes::StreamingData(_data) => {
                    if let Err(e) = processed_data_tx.send(Tables::default()) {
                        log::error!("{name}: cannot send data over data_tx: {e}");
                    }
                }
            },
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => {
                // don't log error message if the channel was disconnected because of cancellation request
                if !cancellation_token.is_cancelled() {
                    log::error!("vehicle_data_rx channel disconnected, exiting {name}");
                }
                break;
            }
        }
        if cancellation_token.is_cancelled() {
            break;
        }
    }
    tracing::warn!("exiting {name}");
}
