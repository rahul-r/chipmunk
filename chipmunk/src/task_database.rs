use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::database;
use crate::database::tables::Tables;
use crate::tasks::{DatabaseDataType, DatabaseRespType};
use tokio_util::sync::CancellationToken;

pub async fn database_task(
    mut data_rx: mpsc::Receiver<DatabaseDataType>,
    data_resp_tx: mpsc::Sender<DatabaseRespType>,
    config: Config,
    cancellation_token: CancellationToken,
    pool: &sqlx::PgPool,
) {
    use mpsc::error::*;
    let name = "database_task";

    let car_data_database_url = config
        .car_data_database_url
        .lock()
        .map(|c| c.get())
        .map_err(|e| log::error!("Error reading `car_data_database_url` from config: {e}"))
        .ok()
        .flatten();

    let car_data_db_pool = if let Some(ref url) = car_data_database_url {
        log::info!("Connecting to car data database `{url}`");
        PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_millis(3000))
            .connect(url)
            .await
            .map_err(|e| log::error!("Error connecting to `{car_data_database_url:?}`: {e}"))
            .ok()
    } else {
        None
    };

    loop {
        match data_rx.try_recv() {
            Ok(data) => match data {
                DatabaseDataType::RawData(d) => {
                    if let Some(ref car_data_pool) = car_data_db_pool {
                        if let Err(e) =
                            database::tables::vehicle_data::db_insert_json(&d, car_data_pool).await
                        {
                            log::error!("Error logging to `{car_data_database_url:?}`: {e}");
                        };
                    }
                    if let Err(e) = database::tables::vehicle_data::db_insert_json(&d, pool).await {
                        log::error!("{e}");
                    };
                }
                DatabaseDataType::Tables(table_list) => {
                    let mut last_tables = Tables::default();
                    for t in table_list {
                        match t.db_insert(pool).await {
                            Ok(updated_tables) => last_tables = updated_tables,
                            Err(e) => log::error!("Error inserting tables into database: {:?}", e),
                        }
                    }
                    if let Err(e) = data_resp_tx
                        .send(DatabaseRespType::Tables(last_tables))
                        .await
                    {
                        log::error!("Error sending response from database task: {e}");
                    }
                }
            },
            Err(TryRecvError::Disconnected) => {
                // don't log error message if the channel was closed because of a cancellation request
                if !cancellation_token.is_cancelled() {
                    log::error!("data_rx channel closed, exiting {name}");
                }
                break;
            }
            Err(TryRecvError::Empty) => (),
        }
        if cancellation_token.is_cancelled() {
            break;
        }
        tokio::task::yield_now().await;
    }
    tracing::warn!("exiting {name}");
}
