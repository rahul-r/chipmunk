use sqlx::PgPool;

use super::charging::{Charges, ChargingProcess, ChargeStat};

impl ChargingProcess {
    pub async fn db_load_last(pool: &PgPool) -> sqlx::Result<Self> {
        let cp = sqlx::query_as!(Self,
            r#"
                SELECT
                    id,
                    start_date,
                    end_date,
                    charge_energy_added,
                    start_ideal_range_km,
                    end_ideal_range_km,
                    start_battery_level,
                    end_battery_level,
                    duration_min,
                    outside_temp_avg,
                    car_id,
                    position_id,
                    address_id,
                    start_rated_range_km,
                    end_rated_range_km,
                    geofence_id,
                    charge_energy_used,
                    cost,
                    charging_status AS "charging_status!: ChargeStat"
                FROM charging_processes
                ORDER BY start_date DESC LIMIT 1
            "#)
            .fetch_one(pool)
            .await?;
        Ok(cp)
    }

    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i32> {
        let id = sqlx::query!(
            r#"
        INSERT INTO charging_processes
        (
            start_date,
            end_date,
            charge_energy_added,
            start_ideal_range_km,
            end_ideal_range_km,
            start_battery_level,
            end_battery_level,
            duration_min,
            outside_temp_avg,
            car_id,
            position_id,
            address_id,
            start_rated_range_km,
            end_rated_range_km,
            geofence_id,
            charge_energy_used,
            cost,
            charging_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        RETURNING id"#,
            self.start_date,
            self.end_date,
            self.charge_energy_added,
            self.start_ideal_range_km,
            self.end_ideal_range_km,
            self.start_battery_level,
            self.end_battery_level,
            self.duration_min,
            self.outside_temp_avg,
            self.car_id,
            self.position_id,
            self.address_id,
            self.start_rated_range_km,
            self.end_rated_range_km,
            self.geofence_id,
            self.charge_energy_used,
            self.cost,
            self.charging_status.clone() as ChargeStat,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id)
    }

    pub async fn db_update(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
        UPDATE charging_processes
        SET
            start_date = $1,
            end_date = $2,
            charge_energy_added = $3,
            start_ideal_range_km = $4,
            end_ideal_range_km = $5,
            start_battery_level = $6,
            end_battery_level = $7,
            duration_min = $8,
            outside_temp_avg = $9,
            position_id = $10,
            address_id = $11,
            start_rated_range_km = $12,
            end_rated_range_km = $13,
            geofence_id = $14,
            charge_energy_used = $15,
            cost = $16,
            charging_status = $17
        WHERE id = $18
        "#,
            self.start_date,
            self.end_date,
            self.charge_energy_added,
            self.start_ideal_range_km,
            self.end_ideal_range_km,
            self.start_battery_level,
            self.end_battery_level,
            self.duration_min,
            self.outside_temp_avg,
            self.position_id,
            self.address_id,
            self.start_rated_range_km,
            self.end_rated_range_km,
            self.geofence_id,
            self.charge_energy_used,
            self.cost,
            self.charging_status.clone() as ChargeStat,
            self.id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // pub async fn db_get(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
    //     Ok(
    //         sqlx::query_as!(Self, r#"SELECT * FROM charging_processes"#)
    //             .fetch_all(pool)
    //             .await?,
    //     )
    // }
}

impl Charges {
    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO charges
            (
                date,
                battery_heater_on,
                battery_level,
                charge_energy_added,
                charger_actual_current,
                charger_phases,
                charger_pilot_current,
                charger_power,
                charger_voltage,
                fast_charger_present,
                conn_charge_cable,
                fast_charger_brand,
                fast_charger_type,
                ideal_battery_range_km,
                not_enough_power_to_heat,
                outside_temp,
                charging_process_id,
                battery_heater,
                battery_heater_no_power,
                rated_battery_range_km,
                usable_battery_level
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)"#,
            self.date,
            self.battery_heater_on,
            self.battery_level,
            self.charge_energy_added,
            self.charger_actual_current,
            self.charger_phases,
            self.charger_pilot_current,
            self.charger_power,
            self.charger_voltage,
            self.fast_charger_present,
            self.conn_charge_cable,
            self.fast_charger_brand,
            self.fast_charger_type,
            self.ideal_battery_range_km,
            self.not_enough_power_to_heat,
            self.outside_temp,
            self.charging_process_id,
            self.battery_heater,
            self.battery_heater_no_power,
            self.rated_battery_range_km,
            self.usable_battery_level
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

pub async fn get_charges_for_charging_process(
    pool: &PgPool,
    charging_process_id: i32,
) -> sqlx::Result<Vec<Charges>> {
    Ok(sqlx::query_as!(
        Charges,
        r#"SELECT * FROM charges WHERE charging_process_id = $1"#,
        charging_process_id
    )
    .fetch_all(pool)
    .await?)
}
