use sqlx::PgPool;
use chrono::NaiveDateTime;

use crate::{database::types::ChargeStat, utils::time_diff_minutes_i64};
use super::{charges::Charges, DBTable};
use crate::charging::calculate_cost;

#[derive(Debug, Default, Clone, PartialEq, sqlx::FromRow)]
pub struct ChargingProcess {
    pub id: i32,
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
    pub charge_energy_added: Option<f32>,
    pub start_ideal_range_km: Option<f32>,
    pub end_ideal_range_km: Option<f32>,
    pub start_battery_level: Option<i16>,
    pub end_battery_level: Option<i16>,
    pub duration_min: Option<i16>,
    pub outside_temp_avg: Option<f32>,
    pub car_id: i16,
    pub position_id: i32,
    pub address_id: Option<i32>,
    pub start_rated_range_km: Option<f32>,
    pub end_rated_range_km: Option<f32>,
    pub geofence_id: Option<i32>,
    pub charge_energy_used: Option<f32>,
    pub cost: Option<f32>,
    pub charging_status: ChargeStat, // This is used to track the current status of charging
}

impl ChargingProcess {
    pub fn from_charges(
        // Merge this with the start() function
        charge_start: &Charges,
        charge_end: &Charges,
        car_id: i16,
        position_id: i32,
        address_id: Option<i32>,
        geofence_id: Option<i32>,
    ) -> anyhow::Result<Self> {
        let charging_process = ChargingProcess {
            id: 0,
            start_date: charge_start.date.unwrap_or_default(),
            end_date: charge_end.date,
            charge_energy_added: charge_end.charge_energy_added,
            start_ideal_range_km: charge_start.ideal_battery_range_km,
            end_ideal_range_km: charge_end.ideal_battery_range_km,
            start_battery_level: charge_start.battery_level,
            end_battery_level: charge_end.battery_level,
            duration_min: time_diff_minutes_i64(charge_start.date, charge_end.date)
                .map(|x| x as i16),
            outside_temp_avg: charge_end
                .outside_temp
                .zip(charge_start.outside_temp)
                .map(|(a, b)| (a + b) / 2.0),
            start_rated_range_km: charge_start.rated_battery_range_km,
            end_rated_range_km: charge_end.rated_battery_range_km,
            geofence_id,
            charge_energy_used: None,
            cost: calculate_cost(charge_start),
            car_id,
            position_id,
            address_id,
            charging_status: ChargeStat::Done,
        };

        Ok(charging_process)
    }

    pub fn start(
        charge_start: &Charges,
        car_id: i16,
        position_id: i32,
        address_id: Option<i32>,
        geofence_id: Option<i32>,
    ) -> Self {
        ChargingProcess {
            start_date: charge_start.date.unwrap_or_default(),
            charge_energy_added: charge_start.charge_energy_added,
            start_ideal_range_km: charge_start.ideal_battery_range_km,
            start_battery_level: charge_start.battery_level,
            duration_min: Some(0),
            outside_temp_avg: charge_start.outside_temp,
            start_rated_range_km: charge_start.rated_battery_range_km,
            geofence_id,
            cost: calculate_cost(charge_start),
            car_id,
            position_id,
            address_id,
            charging_status: ChargeStat::Start,
            ..Self::default()
        }
    }

    pub fn update(&self, charges: &Charges) -> Self {
        Self {
            charge_energy_added: charges.charge_energy_added,
            duration_min: time_diff_minutes_i64(Some(self.start_date), charges.date)
                .map(|x| x as i16),
            outside_temp_avg: self
                .outside_temp_avg
                .zip(charges.outside_temp)
                .map(|(a, b)| (a + b) / 2.0),
            cost: calculate_cost(charges),
            end_date: charges.date,
            end_ideal_range_km: charges.ideal_battery_range_km,
            end_battery_level: charges.battery_level,
            end_rated_range_km: charges.rated_battery_range_km,
            charging_status: ChargeStat::Charging,
            ..self.clone()
        }
    }

    pub fn reset(&self) -> Self {
        Self {
            id: self.id,
            charging_status: ChargeStat::Done,
            ..Self::default()
        }
    }
}

impl DBTable for ChargingProcess {
    fn table_name() -> &'static str {
        "charging_processes"
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        let cp = sqlx::query_as!(
            Self,
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
            "#
        )
        .fetch_one(pool)
        .await?;
        Ok(cp)
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
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

        Ok(id as i64)
    }

    async fn db_update(&self, pool: &PgPool) -> sqlx::Result<()> {
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
}
