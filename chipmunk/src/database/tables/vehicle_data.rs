use chrono::Utc;
use serde_json;
use sqlx::PgPool;
use std::ops::{Deref, DerefMut};
use tesla_api::vehicle_data::VehicleData;

use super::DBTable;

#[derive(sqlx::FromRow, Debug)]
pub struct VehicleDataRow {
    pub data: sqlx::types::Json<VehicleData>,
}

impl Deref for VehicleDataRow {
    type Target = VehicleData;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for VehicleDataRow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl VehicleDataRow {
    pub fn get_data(&self) -> VehicleData {
        self.data.as_ref().clone()
    }
}

pub async fn num_car_data_rows(pool: &PgPool) -> sqlx::Result<i64> {
    Ok(sqlx::query!(r#"SELECT count(timestamp) FROM car_data"#)
        .fetch_one(pool)
        .await?
        .count
        .unwrap_or(0))
}

/**
* Get list of car_data from the database
* This function will fetch `num_rows_to_fetch` number of data points starting at offset `starting_row`
*/
#[allow(dead_code)]
pub async fn db_get(
    pool: &PgPool,
    batch_size: i64,
    row_offset: i64,
) -> sqlx::Result<Vec<VehicleDataRow>> {
    sqlx::query_as!(
            VehicleDataRow,
            r#"SELECT data as "data!:sqlx::types::Json<VehicleData>" FROM car_data ORDER BY timestamp ASC LIMIT $1 OFFSET $2"#,
            batch_size,
            row_offset
        )
        .fetch_all(pool)
        .await
}

#[allow(dead_code)]
pub async fn db_get_between(
    pool: &PgPool,
    start_time: i64,
    end_time: i64,
) -> sqlx::Result<Vec<VehicleDataRow>> {
    sqlx::query_as!(
            VehicleDataRow,
            r#"SELECT data as "data!:sqlx::types::Json<VehicleData>" FROM car_data WHERE timestamp BETWEEN $1 AND $2 ORDER BY timestamp ASC"#,
            start_time,
            end_time
        )
        .fetch_all(pool)
        .await
}

pub async fn db_insert_json(data: &str, pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(r#"INSERT INTO car_data (timestamp,data) VALUES ($1, $2::json)"#)
        .bind(Utc::now().timestamp_millis())
        .bind(data)
        .execute(pool)
        .await?;

    Ok(())
}

impl DBTable for VehicleData {
    fn table_name() -> &'static str {
        "car_data"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let Some(timestamp) = self.timestamp_epoch() else {
            return Err(sqlx::Error::Protocol(
                "No timestamp found in vehicle data".into(),
            ));
        };

        let data_json = match serde_json::to_value(self) {
            Ok(val) => val,
            Err(e) => {
                return Err(sqlx::Error::Protocol(format!(
                    "Error converting vehicle data to JSON: {}",
                    e
                )))
            }
        };

        let timestamp = sqlx::query!(
            r#"INSERT INTO car_data (timestamp,data) VALUES ($1, $2) RETURNING timestamp"#,
            timestamp as i64,
            data_json,
        )
        .fetch_one(pool)
        .await?
        .timestamp;

        Ok(timestamp) // vehicle_data table doesn't have the id field. return timestamp instead
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<VehicleData> {
        // TODO: filter car_data by car_id
        sqlx::query_as!(
            VehicleDataRow,
            r#"SELECT data as "data!:sqlx::types::Json<VehicleData>" FROM car_data ORDER BY timestamp DESC LIMIT 1"#,
        )
        .fetch_one(pool)
        .await
        .map(|d| d.get_data())
    }
}

#[tokio::test]
async fn test_vehicle_data_insertion() {
    dotenvy::dotenv().ok();
    let url = &std::env::var("TEST_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TEST_DATABASE_URL`");
    let pool = crate::database::initialize(url)
        .await
        .expect("Error initializing database");

    let vehicle_state = tesla_api::vehicle_data::VehicleState {
        timestamp: Some(78645),
        ..tesla_api::vehicle_data::VehicleState::default()
    };
    let data = VehicleData {
        vehicle_state: Some(vehicle_state),
        ..VehicleData::default()
    };
    data.db_insert(&pool)
        .await
        .expect("Error inserting vehicle data");
}
