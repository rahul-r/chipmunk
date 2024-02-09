use sqlx::PgPool;

use crate::database::tables::drive::Drive;

use super::Teslamate;

impl Teslamate for Drive {
    fn table_name() -> &'static str {
        "drives"
    }

    async fn tm_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        let tm_drive: Drive = sqlx::query_as(
            r#"
            SELECT
                id,
                start_date,
                end_date,
                outside_temp_avg::FLOAT4,
                speed_max::FLOAT4,
                power_max::FLOAT4,
                power_min::FLOAT4,
                start_ideal_range_km::FLOAT4,
                end_ideal_range_km::FLOAT4,
                start_km::FLOAT4,
                end_km::FLOAT4,
                distance::FLOAT4,
                duration_min,
                car_id,
                inside_temp_avg::FLOAT4,
                start_address_id,
                end_address_id,
                start_rated_range_km::FLOAT4,
                end_rated_range_km::FLOAT4,
                start_position_id,
                end_position_id,
                start_geofence_id,
                end_geofence_id,
                false as in_progress
            FROM drives
            ORDER BY id DESC LIMIT 1
            "#
        )
        .fetch_one(pool)
        .await?;
        Ok(tm_drive)
    }

    async fn tm_get_id(pool: &PgPool, id: i64) -> sqlx::Result<Self> {
        let tm_drive: Drive = sqlx::query_as(
            r#"
            SELECT
                id,
                start_date,
                end_date,
                outside_temp_avg::FLOAT4,
                speed_max::FLOAT4,
                power_max::FLOAT4,
                power_min::FLOAT4,
                start_ideal_range_km::FLOAT4,
                end_ideal_range_km::FLOAT4,
                start_km::FLOAT4,
                end_km::FLOAT4,
                distance::FLOAT4,
                duration_min,
                car_id,
                inside_temp_avg::FLOAT4,
                start_address_id,
                end_address_id,
                start_rated_range_km::FLOAT4,
                end_rated_range_km::FLOAT4,
                start_position_id,
                end_position_id,
                start_geofence_id,
                end_geofence_id,
                false as in_progress
            FROM drives
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Ok(tm_drive)
    }
}