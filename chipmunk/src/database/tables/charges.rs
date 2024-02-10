use sqlx::PgPool;

use super::DBTable;

use anyhow::Context;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tesla_api::{
    utils::{miles_to_km, timestamp_to_naivedatetime},
    vehicle_data::VehicleData,
};


#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow)]
pub struct Charges {
    pub id: i32,
    pub date: Option<NaiveDateTime>,
    pub battery_heater_on: Option<bool>,
    pub battery_level: Option<i16>,
    pub charge_energy_added: Option<f32>,
    pub charger_actual_current: Option<i16>,
    pub charger_phases: Option<i16>,
    pub charger_pilot_current: Option<i16>,
    pub charger_power: Option<i16>,
    pub charger_voltage: Option<i16>,
    pub fast_charger_present: Option<bool>,
    pub conn_charge_cable: Option<String>,
    pub fast_charger_brand: Option<String>,
    pub fast_charger_type: Option<String>,
    pub ideal_battery_range_km: Option<f32>,
    pub not_enough_power_to_heat: Option<bool>,
    pub outside_temp: Option<f32>,
    pub charging_process_id: i32,
    pub battery_heater: Option<bool>,
    pub battery_heater_no_power: Option<bool>,
    pub rated_battery_range_km: Option<f32>,
    pub usable_battery_level: Option<i16>,
}

impl Charges {
    pub fn from(data: &VehicleData, charging_process_id: i32) -> anyhow::Result<Self> {
        let charge_state = data.charge_state.clone().context("charge_state is None")?;
        let climate_state = data
            .climate_state
            .clone()
            .context("climate_state is None")?;
        Ok(Self {
            id: 0,
            date: timestamp_to_naivedatetime(charge_state.timestamp),
            battery_heater_on: charge_state.battery_heater_on,
            battery_level: charge_state.battery_level,
            charge_energy_added: charge_state.charge_energy_added,
            charger_actual_current: charge_state.charger_actual_current,
            charger_phases: charge_state.charger_phases,
            charger_pilot_current: charge_state.charger_pilot_current,
            charger_power: charge_state.charger_power,
            charger_voltage: charge_state.charger_voltage,
            fast_charger_present: charge_state.fast_charger_present,
            conn_charge_cable: charge_state.conn_charge_cable,
            fast_charger_brand: charge_state.fast_charger_brand,
            fast_charger_type: charge_state.fast_charger_type,
            ideal_battery_range_km: miles_to_km(&charge_state.ideal_battery_range),
            not_enough_power_to_heat: charge_state.not_enough_power_to_heat,
            outside_temp: climate_state.outside_temp,
            charging_process_id,
            battery_heater: climate_state.battery_heater,
            battery_heater_no_power: climate_state.battery_heater_no_power,
            rated_battery_range_km: miles_to_km(&charge_state.battery_range),
            usable_battery_level: charge_state.usable_battery_level,
        })
    }

    /// Get the list of charges associated with a charging process
    pub async fn for_charging_process(
        pool: &PgPool,
        charging_process_id: i32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Charges,
            r#"SELECT * FROM charges WHERE charging_process_id = $1"#,
            charging_process_id
        )
        .fetch_all(pool)
        .await
    }

    /// Insert a charge into the database for the last charging process
    pub async fn db_insert_for_last_charging_process(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, (SELECT id FROM charging_processes ORDER BY id DESC LIMIT 1), $17, $18, $19, $20)
            RETURNING id"#,
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
            self.battery_heater,
            self.battery_heater_no_power,
            self.rated_battery_range_km,
            self.usable_battery_level
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }
}

impl DBTable for Charges {
    fn table_name() -> &'static str {
        "charges"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            RETURNING id"#,
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
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, r#"SELECT * FROM charges ORDER BY id DESC LIMIT 1"#)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            log::error!("Error getting last row from table `{}`: {}", Self::table_name(), e);
            e
        })
    }

    async fn db_get_all(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT * FROM charges ORDER BY id ASC"#)
            .fetch_all(pool)
            .await
    }
}
