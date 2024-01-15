use anyhow::Context;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

use sqlx::PgPool;
use tesla_api::vehicle_data::VehicleData;

use crate::utils::capitalize_string_option;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Car {
    pub id: i16,
    pub eid: i64,
    pub vid: i64,
    pub model: Option<String>,
    pub efficiency: Option<f64>,
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
        let model_code = Self::get_model_code(&vehicle_config.car_type);
        let car = Self {
            id: 0,
            eid: Self::convert_id(data.id, "id")?,
            vid: Self::convert_id(data.vehicle_id, "vehicle_id")?,
            model: model_code.clone(),
            efficiency: None, // TODO: Calculate efficiency. See teslamate code lib/teslamate/log.ex for details
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
            marketing_name: Self::get_marketing_name(
                model_code,
                vehicle_config.trim_badging,
                vehicle_config.car_type,
            ),
        };
        Ok(car)
    }
    fn get_model_code(model_name: &Option<String>) -> Option<String> {
        let Some(name) = model_name else {
            log::warn!("model_name is `None`");
            return None;
        };

        let model_code = match name.to_lowercase().as_str() {
            "models" | "lychee" => "S",
            "model3" => "3",
            "modelx" | "tamarind" => "X",
            "modely" => "Y",
            s => {
                log::warn!("Unknown model name `{s}`");
                return None;
            }
        };

        Some(model_code.to_string())
    }

    fn get_marketing_name(
        model: Option<String>,
        trim_badging: Option<String>,
        m_type: Option<String>,
    ) -> Option<String> {
        let Some(model) = model else {
            log::warn!("Model is `None`");
            return None;
        };

        let Some(trim_badging) = trim_badging else {
            // log::warn!("trim_badging is `None`"); // TODO: uncomment this
            return None;
        };

        let Some(m_type) = m_type else {
            log::warn!("Model type is `None`");
            return None;
        };

        let model = model.to_ascii_uppercase();
        let trim_badging = trim_badging.to_ascii_uppercase();
        let m_type = m_type.to_ascii_lowercase();

        let marketing_name = match (model.as_str(), trim_badging.as_str(), m_type.as_str()) {
            ("S", "100D", "lychee") => "LR",
            ("S", "P100D", "lychee") => "Plaid",
            ("3", "P74D", _) => "LR AWD Performance",
            ("3", "74D", _) => "LR AWD",
            ("3", "74", _) => "LR",
            ("3", "62", _) => "MR",
            ("3", "50", _) => "SR+",
            ("X", "100D", "tamarind") => "LR",
            ("X", "P100D", "tamarind") => "Plaid",
            ("Y", "P74D", _) => "LR AWD Performance",
            ("Y", "74D", _) => "LR AWD",
            (m, tr, ty) => {
                log::warn!(
                    "Unknown combination of model `{m}`, trim_badging `{tr}`, and type `{ty}`"
                );
                return None;
            }
        };

        Some(marketing_name.to_string())
    }

    fn convert_id(id: Option<u64>, name: &str) -> anyhow::Result<i64> {
        match id {
            Some(i) => Ok(i.try_into()?),
            None => anyhow::bail!("{name} is None"),
        }
    }

    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
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

    /// Get the list of cars from the database.
    pub async fn db_get(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(Self, r#"SELECT * FROM cars"#)
            .fetch_all(pool)
            .await
    }

    /// Get the car from the database by looking at the ID.
    pub async fn db_get_car_by_id(pool: &PgPool, id: i16) -> sqlx::Result<Self> {
        let cars = sqlx::query_as!(Self, r#"SELECT * FROM cars where id = $1"#, id)
            .fetch_all(pool)
            .await?;

        if cars.len() > 1 {
            log::error!("More than one car found with id `{}`, using the last car from the list of cars", id);
        }

        if let Some(car) = cars.last() {
            return Ok(car.clone());
        } else {
            log::error!("No car found with id `{}`", id);
            return Err(sqlx::Error::RowNotFound);
        }
    }
}

#[derive(Debug)]
pub struct CarSettings {
    pub id: i64,
    pub suspend_min: i32,
    pub suspend_after_idle_min: i32,
    pub req_not_unlocked: bool,
    pub free_supercharging: bool,
    pub use_streaming_api: bool,
}

impl Default for CarSettings {
    fn default() -> Self {
        Self {
            id: 0,
            suspend_min: 21,
            suspend_after_idle_min: 15,
            req_not_unlocked: false,
            free_supercharging: false,
            use_streaming_api: true,
        }
    }
}

impl CarSettings {
    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO car_settings
        (
            suspend_min,
            suspend_after_idle_min,
            req_not_unlocked,
            free_supercharging,
            use_streaming_api
        )
        VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE
                SET
                    suspend_min = excluded.suspend_min,
                    suspend_after_idle_min = excluded.suspend_after_idle_min,
                    req_not_unlocked = excluded.req_not_unlocked,
                    free_supercharging = excluded.free_supercharging,
                    use_streaming_api = excluded.use_streaming_api
        RETURNING id"#,
            self.suspend_min,
            self.suspend_after_idle_min,
            self.req_not_unlocked,
            self.free_supercharging,
            self.use_streaming_api,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id)
    }

    // pub async fn db_get(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
    //     Ok(
    //         sqlx::query_as!(CarSettings, r#"SELECT * FROM car_settings"#)
    //             .fetch_all(pool)
    //             .await?,
    //     )
    // }

    // pub async fn db_get_for_car(pool: &PgPool, settings_id: i64) -> sqlx::Result<Vec<Self>> {
    //     Ok(sqlx::query_as!(
    //         CarSettings,
    //         r#"SELECT * FROM car_settings WHERE id = $1"#,
    //         settings_id
    //     )
    //     .fetch_all(pool)
    //     .await?)
    // }
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
        // Car not found in the databse, insert it
        let car_settings_id = CarSettings::default().db_insert(pool).await?;
        let car = Car::from(&vehicle_data, car_settings_id)?;
        car_id = car.db_insert(pool).await? as i16;
        let updated_cars = Car::db_get(pool).await?;
        return Ok((updated_cars, car_id));
    }

    Ok((cars, car_id))
}
