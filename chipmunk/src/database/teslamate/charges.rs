use crate::database::tables::charges::Charges;

impl Charges {
    pub async fn tm_get_for_charging(pool: &sqlx::PgPool, cp_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Charges>(
            r#"
            SELECT
                id,
                date,
                battery_heater_on,
                battery_level,
                charge_energy_added::FLOAT4,
                charger_actual_current,
                charger_phases,
                charger_pilot_current,
                charger_power,
                charger_voltage,
                fast_charger_present,
                conn_charge_cable,
                fast_charger_brand,
                fast_charger_type,
                ideal_battery_range_km::FLOAT4,
                not_enough_power_to_heat,
                outside_temp::FLOAT4,
                charging_process_id,
                battery_heater,
                battery_heater_no_power,
                rated_battery_range_km::FLOAT4,
                usable_battery_level
            FROM charges WHERE charging_process_id = $1
            ORDER BY date ASC
        "#,
        )
        .bind(cp_id)
        .fetch_all(pool)
        .await
    }
}
