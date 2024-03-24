use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use super::DBTable;

#[derive(Debug, Default, Clone)]
pub struct SoftwareUpdate {
    pub id: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub version: String,
    pub car_id: i16,
}

pub fn software_updated(previous_data: Option<&VehicleData>, current_data: &VehicleData) -> bool {
    let version = |data: Option<&VehicleData>| -> anyhow::Result<String> {
        data.context("vehicle_data in None")?
            .vehicle_state
            .as_ref()
            .context("vehicle_state in None")?
            .car_version
            .clone()
            .context("car_version is None")
    };

    if let (Ok(previous), Ok(current)) = (version(previous_data), version(Some(current_data))) {
        return previous != current;
    }

    false
}

impl DBTable for SoftwareUpdate {
    fn table_name() -> &'static str {
        "updates"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO updates
        (
            start_date,
            end_date,
            version,
            car_id
        )
        VALUES ($1, $2, $3, $4)
        RETURNING id"#,
            self.start_date,
            self.end_date,
            self.version,
            self.car_id,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }
}

pub async fn insert_end_date(pool: &PgPool, data: SoftwareUpdate) -> sqlx::Result<i64> {
    sqlx::query!(
        r#"UPDATE updates SET end_date = $2 WHERE id = $1"#,
        data.id,
        data.end_date
    )
    .execute(pool)
    .await?;

    Ok(0)
}
