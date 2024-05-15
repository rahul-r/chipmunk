use crate::database::tables::position::Position;
use crate::database::DBTable;

impl Position {
    pub async fn tm_get_for_drive(
        pool: &sqlx::PgPool,
        car_id: i16,
        drive_id: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Position>(
            format!(
                r#"SELECT
                    id,
                    date,
                    latitude::FLOAT4,
                    longitude::FLOAT4,
                    speed::FLOAT4,
                    power::FLOAT4,
                    odometer::FLOAT4,
                    ideal_battery_range_km::FLOAT4,
                    battery_level,
                    outside_temp::FLOAT4,
                    elevation::FLOAT4,
                    fan_status,
                    driver_temp_setting::FLOAT4,
                    passenger_temp_setting::FLOAT4,
                    is_climate_on,
                    is_rear_defroster_on,
                    is_front_defroster_on,
                    car_id,
                    drive_id,
                    inside_temp::FLOAT4,
                    battery_heater,
                    battery_heater_on,
                    battery_heater_no_power,
                    est_battery_range_km::FLOAT4,
                    rated_battery_range_km::FLOAT4,
                    usable_battery_level,
                    tpms_pressure_fl::FLOAT4,
                    tpms_pressure_fr::FLOAT4,
                    tpms_pressure_rl::FLOAT4,
                    tpms_pressure_rr::FLOAT4
                FROM {} WHERE drive_id = {} AND car_id = {}
                ORDER BY date ASC"#,
                Self::table_name(),
                drive_id,
                car_id
            )
            .as_str(),
        )
        .fetch_all(pool)
        .await
    }
}
