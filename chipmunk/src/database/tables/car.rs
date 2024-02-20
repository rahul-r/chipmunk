use std::collections::HashMap;

use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use crate::utils::capitalize_string_option;

use super::{car_settings::CarSettings, DBTable};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Car {
    pub id: i16,
    pub eid: i64,
    pub vid: i64,
    pub model: Option<String>,
    pub efficiency: Option<f32>,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub vin: Option<String>,
    pub name: Option<String>,
    pub trim_badging: Option<String>,
    pub settings_id: i64,
    pub exterior_color: Option<String>,
    pub spoiler_type: Option<String>,
    pub wheel_type: Option<String>,
    pub display_priority: i16,
    pub marketing_name: Option<String>,
}

impl Car {
    pub fn from(data: &VehicleData, settings_id: i64) -> anyhow::Result<Self> {
        let vehicle_config = data
            .vehicle_config
            .clone()
            .context("vehicle_config is None")?;
        let model_code = tesla_api::Vehicle::get_model_code(&vehicle_config.car_type);
        let car = Self {
            id: 0,
            eid: Self::convert_id(data.id, "id")?,
            vid: Self::convert_id(data.vehicle_id, "vehicle_id")?,
            model: model_code.clone(),
            efficiency: None,
            inserted_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            vin: data.vin.clone(),
            name: data
                .vehicle_state
                .as_ref()
                .and_then(|vs| vs.vehicle_name.clone())
                .or(Some("Unknown Vehicle".to_string())),
            trim_badging: capitalize_string_option(vehicle_config.trim_badging.clone()),
            settings_id,
            exterior_color: vehicle_config.exterior_color,
            spoiler_type: vehicle_config.spoiler_type,
            wheel_type: vehicle_config.wheel_type,
            // display_priority field is used to customize the displayed order of vehicles on
            // Grafana dashboards. See https://github.com/adriankumpf/teslamate/pull/1904 for
            // details
            display_priority: 1,
            marketing_name: tesla_api::Vehicle::get_marketing_name(
                model_code,
                vehicle_config.trim_badging,
                vehicle_config.car_type,
            ),
        };
        Ok(car)
    }

    fn convert_id(id: Option<u64>, name: &str) -> anyhow::Result<i64> {
        match id {
            Some(i) => Ok(i.try_into()?),
            None => anyhow::bail!("{name} is None"),
        }
    }

    /// Get the car from the database by looking at the ID.
    pub async fn db_get_car_by_id(pool: &PgPool, id: i16) -> sqlx::Result<Self> {
        let cars = sqlx::query_as!(Self, r#"SELECT * FROM cars where id = $1"#, id)
            .fetch_all(pool)
            .await?;

        if cars.len() > 1 {
            log::error!(
                "More than one car found with id `{}`, using the last car from the list of cars",
                id
            );
        }

        if let Some(car) = cars.last() {
            Ok(car.clone())
        } else {
            log::error!("No car found with id `{}`", id);
            Err(sqlx::Error::RowNotFound)
        }
    }

    /// # Updates the efficiency of a car in the database.
    ///
    /// This function calculates the average efficiency of a car based on its charging processes and
    /// updates the `efficiency` field in the `cars` table.
    ///
    /// The efficiency is calculated as the average of the ratio of `charge_energy_added` to the
    /// difference between the start and end range (either `ideal` or `rated`), for all charging
    /// processes of the car that last more than 10 minutes, end with a battery level of 95 or less,
    /// and add more than 0.0 energy.
    ///
    /// The function uses a SQL query to perform the calculation and update operation in the database.
    /// The query first calculates the efficiency for the selected car in the `charging_processes`
    /// table and stores the results in a temporary table called `efficiency_data`. Then, it updates
    /// the `efficiency` column in the `cars` table with the calculated efficiency values from the
    /// temporary table.
    ///
    /// # Arguments
    ///
    /// * `pool` - A reference to the database connection pool.
    /// * `car_id` - The ID of the car to update efficiency.
    ///
    /// # Returns
    ///
    /// Returns a `Result` indicating the success or failure of the operation. If the operation is
    /// successful, the function returns `Ok(()) and `Err(sqlx::Error)` on failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if the SQL query fails to execute, or if the number of
    /// rows affected by the `UPDATE` operation is not 1.
    pub async fn update_efficiency(pool: &PgPool, car_id: i16) -> sqlx::Result<()> {
        let res = sqlx::query!(
            r#"
            WITH efficiency_data AS (
                SELECT
                    ROUND(
                        AVG(charge_energy_added / NULLIF(
                            CASE
                                WHEN s.preferred_range = 'ideal' THEN cp.end_ideal_range_km
                                WHEN s.preferred_range = 'rated' THEN cp.end_rated_range_km
                                ELSE NULL
                            END -
                            CASE
                                WHEN s.preferred_range = 'ideal' THEN cp.start_ideal_range_km
                                WHEN s.preferred_range = 'rated' THEN cp.start_rated_range_km
                                ELSE NULL
                            END, 0))::numeric, 5
                    )::FLOAT4 AS efficiency
                FROM
                    charging_processes cp,
                    settings s
                WHERE
                    cp.car_id = $1
                    AND cp.duration_min > 10
                    AND cp.end_battery_level <= 95
                    AND cp.charge_energy_added > 0.0
                    AND s.id = 1
            )
            UPDATE cars
            SET efficiency = efficiency_data.efficiency
            FROM efficiency_data
            WHERE cars.id = $1
            "#,
            car_id
        )
        .execute(pool)
        .await?;

        if res.rows_affected() != 1 {
            let msg = format!("Error updating efficiency. Expected to update 1 row, but updated {} rows", res.rows_affected());
            log::error!("{}", msg);
            Err(sqlx::Error::Protocol(msg))
        } else {
            Ok(())
        }
    }
}

impl DBTable for Car {
    fn table_name() -> &'static str {
        "cars"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO cars
        (
            eid,
            vid,
            model,
            efficiency,
            inserted_at,
            updated_at,
            vin,
            name,
            trim_badging,
            settings_id,
            exterior_color,
            spoiler_type,
            wheel_type,
            display_priority,
            marketing_name
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (vid) DO UPDATE
                SET
                    eid = excluded.eid,
                    vid = excluded.vid,
                    model = excluded.model,
                    efficiency = excluded.efficiency,
                    updated_at = excluded.updated_at,
                    vin = excluded.vin,
                    name = excluded.name,
                    trim_badging = excluded.trim_badging,
                    settings_id = excluded.settings_id,
                    exterior_color = excluded.exterior_color,
                    spoiler_type = excluded.spoiler_type,
                    wheel_type = excluded.wheel_type,
                    display_priority = excluded.display_priority,
                    marketing_name = excluded.marketing_name
        RETURNING id"#,
            self.eid,
            self.vid,
            self.model,
            self.efficiency,
            self.inserted_at,
            self.updated_at,
            self.vin,
            self.name,
            self.trim_badging,
            self.settings_id,
            self.exterior_color,
            self.spoiler_type,
            self.wheel_type,
            self.display_priority,
            self.marketing_name
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, r#"SELECT * FROM cars ORDER BY id DESC LIMIT 1"#)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                log::error!(
                    "Error getting last row from table `{}`: {}",
                    Self::table_name(),
                    e
                );
                e
            })
    }

    /// Get the list of cars from the database.
    async fn db_get_all(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT * FROM cars ORDER BY id ASC"#)
            .fetch_all(pool)
            .await
    }
}

/// Get the ID of the car from the database by looking at the VIN. If the car is not in the database, insert it.
///
/// # Arguments
///
/// * `pool` - A reference to the database pool.
/// * `cars` - A vector of `Car` objects.
/// * `vehicle_data` - A reference to the vehicle data.
///
/// # Returns
///
/// Returns a `Result` with a tuple. The tuple contains the updated list of cars and the database ID of the car in vehicle_data.
///
/// This function will return an error if the VIN is not provided or if there is a problem inserting the new car into the database.
pub async fn db_get_or_insert_car(
    pool: &PgPool,
    cars: Vec<Car>,
    vehicle_data: &VehicleData,
) -> anyhow::Result<(Vec<Car>, i16)> {
    let mut car_id = -1;
    if let Some(ref vin) = vehicle_data.vin {
        for car in &cars {
            if car.vin.as_ref() == Some(vin) {
                car_id = car.id;
                break;
            }
        }
    } else {
        anyhow::bail!("VIN is None, cannot get car_id without a valid VIN");
    }

    if car_id == -1 {
        // Car not found in the database, insert it
        let car_settings_id = CarSettings::default().db_insert(pool).await?;
        let car = Car::from(vehicle_data, car_settings_id)?;
        car_id = car.db_insert(pool).await? as i16;
        let updated_cars = Car::db_get_all(pool).await?;
        return Ok((updated_cars, car_id));
    }

    Ok((cars, car_id))
}

/// Get a map of VINs to car IDs from the database
// Read the list of cars from the database, we will check which car the vehicle_data response from the API belongs to
// It is more efficient to store the list of cars in memory and check against it instead of querying the database for each vehicle_data response
pub async fn get_vin_id_map(pool: &PgPool) -> HashMap<String, i16> {
    #[rustfmt::skip]
    let vin_id_map = if let Ok(cars) = Car::db_get_all(pool).await {
        cars
            .iter()
            .filter(|c| c.id > 0 && c.vin.is_some()) // Remove entries with invalid id or None vins
            .map(|c| (c.vin.clone().expect("VIN is None, this should never happen"), c.id)) // Get vin and id from Car struct
            .collect()
    } else {
        log::error!("Error getting cars from database");
        HashMap::new()
    };

    vin_id_map
}

/// Check if the vehicle_data response belongs to a car in the database, if not, insert a new entry and update `vin_id_map`
pub async fn get_car_id_from_vin(
    pool: &PgPool,
    data: &VehicleData,
    vin_id_map: HashMap<String, i16>,
    vin: &String,
) -> (HashMap<String, i16>, Option<i16>) {
    if let Some(id) = vin_id_map.get(vin) {
        return (vin_id_map.clone(), Some(*id));
    }

    log::info!("Vehicle with VIN {vin} not found in the database, inserting a new entry");
    let car_settings_id = match CarSettings::default().db_insert(pool).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("Error inserting car settings into database: {e}");
            return (vin_id_map, None);
        }
    };
    let Ok(car) = Car::from(data, car_settings_id).map_err(|e| log::error!("Error creating car: {e}")) else {
        return (vin_id_map, None);
    };

    let Ok(id) = car.db_insert(pool)
        .await
        .map_err(|e| log::error!("{e}")).map(|id| id as i16)
    else {
        return (vin_id_map, None);
    };

    let mut vin_id_map = vin_id_map;
    vin_id_map.insert(vin.clone(), id);

    (vin_id_map, Some(id))
}

