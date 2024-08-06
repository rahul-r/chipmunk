use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{charges::Charges, DBTable};
use crate::charging::calculate_cost;
use crate::{charging::calculate_energy_used, database::types::ChargeStat};

#[derive(Debug, Default, Clone, PartialEq, sqlx::FromRow)]
pub struct ChargingProcess {
    pub id: i32,
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
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
            duration_min: charge_start
                .date
                .zip(charge_end.date)
                .map(|(st, en)| (en - st).num_minutes())
                .map(|x| x as i16),
            outside_temp_avg: charge_end
                .outside_temp
                .zip(charge_start.outside_temp)
                .map(|(a, b)| (a + b) / 2.0),
            start_rated_range_km: charge_start.rated_battery_range_km,
            end_rated_range_km: charge_end.rated_battery_range_km,
            geofence_id,
            charge_energy_used: Some(0.0),
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
            duration_min: charges
                .date
                .map(|end_date| (self.start_date - end_date).num_minutes() as i16),
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

    async fn db_get_last_id(pool: &PgPool) -> sqlx::Result<i32> {
        let id = sqlx::query!(
            r#"
            SELECT id
            FROM charging_processes
            ORDER BY start_date DESC LIMIT 1
            "#
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id)
    }

    /// Recalculate the last charging process using the list of charges associated with thie charging process
    pub async fn db_recalculate(pool: &PgPool, charge: Option<&Charges>) -> sqlx::Result<()> {
        let id = ChargingProcess::db_get_last_id(pool).await?;
        let charges = Charges::db_get_for_charging_process(pool, id).await?;
        let cost: Option<f32> = charge.and_then(calculate_cost);
        let energy_used = calculate_energy_used(&charges);

        let res = sqlx::query!(
            r#"
            WITH charge_summary AS (
                SELECT 
                    FIRST_VALUE(date) OVER w AS start_date,
                    LAST_VALUE(date) OVER w AS end_date,
                    FIRST_VALUE(battery_level) OVER w AS start_battery_level,
                    LAST_VALUE(battery_level) OVER w AS end_battery_level,
                    FIRST_VALUE(charge_energy_added) OVER w AS start_charge_energy_added,
                    LAST_VALUE(charge_energy_added) OVER w AS end_charge_energy_added,
                    LAST_VALUE(ideal_battery_range_km) OVER w AS end_ideal_range_km,
                    LAST_VALUE(rated_battery_range_km) OVER w AS end_rated_range_km,
                    COALESCE(
                        NULLIF(LAST_VALUE(charge_energy_added) OVER w, 0),
                        MAX(charge_energy_added) OVER w
                    ) - FIRST_VALUE(charge_energy_added) OVER w AS charge_energy_added
                FROM charges
                WHERE charging_process_id = $1
                WINDOW w AS (ORDER BY date RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING)
            ),
            charges_summary AS (
                SELECT 
                    AVG(outside_temp) AS outside_temp_avg
                FROM charges
                WHERE charging_process_id = $1
            )
            UPDATE charging_processes
            SET 
                charge_energy_added = charge_summary.charge_energy_added, 
                end_date = charge_summary.end_date, 
                end_battery_level = charge_summary.end_battery_level, 
                end_rated_range_km = charge_summary.end_rated_range_km,
                end_ideal_range_km = charge_summary.end_ideal_range_km,
                outside_temp_avg = charges_summary.outside_temp_avg,
                duration_min = EXTRACT(EPOCH FROM (charge_summary.end_date - charge_summary.start_date))/60,
                charging_status = $2,
                cost = $3,
                charge_energy_used = $4
            FROM charge_summary CROSS JOIN charges_summary
            WHERE charging_processes.id = $1
            "#,
            id,
            ChargeStat::Done as ChargeStat,
            cost,
            energy_used
        )
        .execute(pool)
        .await?;

        if res.rows_affected() != 1 {
            log::error!(
                "Error updating charging process. Expected to update 1 row, but updated {} rows",
                res.rows_affected()
            );
            Err(sqlx::Error::RowNotFound)
        } else {
            Ok(())
        }
    }
}

impl DBTable for ChargingProcess {
    fn table_name() -> &'static str {
        "charging_processes"
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

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(
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
        .await
    }

    async fn db_get_id(pool: &PgPool, id: i64) -> sqlx::Result<Self> {
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
                WHERE id = $1
            "#,
            id as i32
        )
        .fetch_one(pool)
        .await?;
        Ok(cp)
    }

    async fn db_get_all(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
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
                ORDER BY id ASC
            "#,
        )
        .fetch_all(pool)
        .await
    }
}
