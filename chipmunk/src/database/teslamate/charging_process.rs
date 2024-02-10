use sqlx::{PgPool, Row};

use crate::database::{tables::charging_process::ChargingProcess, types::ChargeStat};

use super::Teslamate;

impl Teslamate for ChargingProcess {
    fn table_name() -> &'static str {
        "charging_processes"
    }

    async fn tm_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        let last_id = sqlx::query("SELECT id FROM charging_processes ORDER BY id DESC LIMIT 1")
            .fetch_one(pool)
            .await?
            .get::<i32, _>("id");
        Self::tm_get_id(pool, last_id as i64).await
    }

    async fn tm_get_id(pool: &PgPool, id: i64) -> sqlx::Result<Self> {
        let res = sqlx::query(r#"
            SELECT
                id,
                start_date,
                end_date,
                charge_energy_added::FLOAT4,
                start_ideal_range_km::FLOAT4,
                end_ideal_range_km::FLOAT4,
                start_battery_level,
                end_battery_level,
                duration_min,
                outside_temp_avg::FLOAT4,
                car_id,
                position_id,
                address_id,
                start_rated_range_km::FLOAT4,
                end_rated_range_km::FLOAT4,
                geofence_id,
                charge_energy_used::FLOAT4,
                cost
            FROM charging_processes WHERE id = $1"#)
            .bind(id as i32)
            .fetch_one(pool)
            .await?;
        Ok(ChargingProcess {
            id: res.get::<i32, _>("id"),
            start_date: res.get::<chrono::NaiveDateTime, _>("start_date"),
            end_date: res.try_get::<chrono::NaiveDateTime, _>("end_date").ok(),
            charge_energy_added: res.try_get::<f32, _>("charge_energy_added").ok(),
            start_ideal_range_km: res.try_get::<f32, _>("start_ideal_range_km").ok(),
            end_ideal_range_km: res.try_get::<f32, _>("end_ideal_range_km").ok(),
            start_battery_level: res.try_get::<i16, _>("start_battery_level").ok(),
            end_battery_level: res.try_get::<i16, _>("end_battery_level").ok(),
            duration_min: res.try_get::<i16, _>("duration_min").ok(),
            outside_temp_avg: res.try_get::<f32, _>("outside_temp_avg").ok(),
            car_id: res.get::<i16, _>("car_id"),
            position_id: res.get::<i32, _>("position_id"),
            address_id: res.try_get::<i32, _>("address_id").ok(),
            start_rated_range_km: res.try_get::<f32, _>("start_rated_range_km").ok(),
            end_rated_range_km: res.try_get::<f32, _>("end_rated_range_km").ok(),
            geofence_id: res.try_get::<i32, _>("geofence_id").ok(),
            charge_energy_used: res.try_get::<f32, _>("charge_energy_used").ok(),
            cost: res.try_get::<f32, _>("cost").ok(),
            charging_status: ChargeStat::Done,
        })
    }
}