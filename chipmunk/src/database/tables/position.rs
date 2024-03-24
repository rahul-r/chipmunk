use anyhow::Context;
use chrono::{DateTime, Utc};

use sqlx::PgPool;
use tesla_api::utils::{get_elevation, miles_to_km, mph_to_kmh, timestamp_to_datetime};
use tesla_api::vehicle_data::VehicleData;

use super::DBTable;

#[derive(Debug, Default, Clone, sqlx::FromRow)]
pub struct Position {
    pub id: Option<i32>,
    pub date: Option<DateTime<Utc>>,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub speed: Option<f32>,
    pub power: Option<f32>,
    pub odometer: Option<f32>, // TODO: rename to odometer_km?
    pub ideal_battery_range_km: Option<f32>,
    pub battery_level: Option<i16>,
    pub outside_temp: Option<f32>,
    pub elevation: Option<f32>,
    pub fan_status: Option<i32>,
    pub driver_temp_setting: Option<f32>,
    pub passenger_temp_setting: Option<f32>,
    pub is_climate_on: Option<bool>,
    pub is_rear_defroster_on: Option<bool>,
    pub is_front_defroster_on: Option<bool>,
    pub car_id: i16,
    pub drive_id: Option<i32>,
    pub inside_temp: Option<f32>,
    pub battery_heater: Option<bool>,
    pub battery_heater_on: Option<bool>,
    pub battery_heater_no_power: Option<bool>,
    pub est_battery_range_km: Option<f32>,
    pub rated_battery_range_km: Option<f32>,
    pub usable_battery_level: Option<i16>,
    pub tpms_pressure_fl: Option<f32>,
    pub tpms_pressure_fr: Option<f32>,
    pub tpms_pressure_rl: Option<f32>,
    pub tpms_pressure_rr: Option<f32>,
}

impl Position {
    pub fn from(data: &VehicleData, car_id: i16, drive_id: Option<i32>) -> anyhow::Result<Self> {
        let charge_state = data.charge_state.clone().context("charge_state is None")?;
        let climate_state = data
            .climate_state
            .clone()
            .context("climate_state is None")?;
        let drive_state = data.drive_state.clone().context("drive_state is None")?;
        let vehicle_state = data
            .vehicle_state
            .clone()
            .context("vehicle_state is None")?;
        Ok(Self {
            id: None,
            date: match timestamp_to_datetime(drive_state.timestamp) {
                None => {
                    log::error!(
                        "Value of `drive_state.timestamp` is None, using current time instead"
                    );
                    Some(chrono::Utc::now())
                }
                time => time,
            },
            latitude: drive_state
                .latitude
                .or(drive_state.latitude)
                .or(drive_state.active_route_latitude)
                .or(drive_state.native_latitude),
            longitude: drive_state
                .longitude
                .or(drive_state.longitude)
                .or(drive_state.active_route_longitude)
                .or(drive_state.native_longitude),
            speed: mph_to_kmh(&drive_state.speed),
            power: drive_state.power,
            odometer: miles_to_km(&vehicle_state.odometer),
            ideal_battery_range_km: miles_to_km(&charge_state.ideal_battery_range),
            battery_level: charge_state.battery_level,
            outside_temp: climate_state.outside_temp,
            elevation: get_elevation(),
            fan_status: climate_state.fan_status,
            driver_temp_setting: climate_state.driver_temp_setting,
            passenger_temp_setting: climate_state.passenger_temp_setting,
            is_climate_on: climate_state.is_climate_on,
            is_rear_defroster_on: climate_state.is_rear_defroster_on,
            is_front_defroster_on: climate_state.is_front_defroster_on,
            car_id,
            drive_id,
            inside_temp: climate_state.inside_temp,
            battery_heater: climate_state.battery_heater,
            battery_heater_on: climate_state.battery_heater_no_power,
            battery_heater_no_power: climate_state.battery_heater_no_power,
            est_battery_range_km: miles_to_km(&charge_state.est_battery_range),
            rated_battery_range_km: miles_to_km(&charge_state.battery_range),
            usable_battery_level: charge_state.usable_battery_level,
            tpms_pressure_fl: vehicle_state.tpms_pressure_fl,
            tpms_pressure_fr: vehicle_state.tpms_pressure_fr,
            tpms_pressure_rl: vehicle_state.tpms_pressure_rl,
            tpms_pressure_rr: vehicle_state.tpms_pressure_rr,
        })
    }

    pub async fn db_update_drive_id(&self, pool: &PgPool, drive_id: i32) -> sqlx::Result<()> {
        sqlx::query!(
            r#"UPDATE positions SET drive_id = $1 WHERE id = $2"#,
            drive_id,
            self.id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn db_get_for_drive(
        pool: &sqlx::PgPool,
        car_id: i16,
        drive_id: i32,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
                SELECT
                    id,
                    date,
                    latitude,
                    longitude,
                    speed,
                    power,
                    odometer,
                    ideal_battery_range_km,
                    battery_level,
                    outside_temp,
                    elevation,
                    fan_status,
                    driver_temp_setting,
                    passenger_temp_setting,
                    is_climate_on,
                    is_rear_defroster_on,
                    is_front_defroster_on,
                    car_id,
                    drive_id,
                    inside_temp,
                    battery_heater,
                    battery_heater_on,
                    battery_heater_no_power,
                    est_battery_range_km,
                    rated_battery_range_km,
                    usable_battery_level,
                    tpms_pressure_fl,
                    tpms_pressure_fr,
                    tpms_pressure_rl,
                    tpms_pressure_rr
                FROM positions WHERE drive_id = $1 AND car_id = $2
                ORDER BY date ASC"#,
            drive_id,
            car_id
        )
        .fetch_all(pool)
        .await
    }
}

impl DBTable for Position {
    fn table_name() -> &'static str {
        "positions"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO positions
        (
            date,
            latitude,
            longitude,
            speed,
            power,
            odometer,
            ideal_battery_range_km,
            battery_level,
            outside_temp,
            elevation,
            fan_status,
            driver_temp_setting,
            passenger_temp_setting,
            is_climate_on,
            is_rear_defroster_on,
            is_front_defroster_on,
            car_id,
            drive_id,
            inside_temp,
            battery_heater,
            battery_heater_on,
            battery_heater_no_power,
            est_battery_range_km,
            rated_battery_range_km,
            usable_battery_level,
            tpms_pressure_fl,
            tpms_pressure_fr,
            tpms_pressure_rl,
            tpms_pressure_rr
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)
            RETURNING id"#,
            self.date,
            self.latitude,
            self.longitude,
            self.speed,
            self.power,
            self.odometer,
            self.ideal_battery_range_km,
            self.battery_level,
            self.outside_temp,
            self.elevation,
            self.fan_status,
            self.driver_temp_setting,
            self.passenger_temp_setting,
            self.is_climate_on,
            self.is_rear_defroster_on,
            self.is_front_defroster_on,
            self.car_id,
            self.drive_id,
            self.inside_temp,
            self.battery_heater,
            self.battery_heater_on,
            self.battery_heater_no_power,
            self.est_battery_range_km,
            self.rated_battery_range_km,
            self.usable_battery_level,
            self.tpms_pressure_fl,
            self.tpms_pressure_fr,
            self.tpms_pressure_rl,
            self.tpms_pressure_rr
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }

    // pub async fn db_get(pool: &PgPool, id: i32) -> sqlx::Result<Self> {
    //     let position = sqlx::query_as::<_, Self>(r#"SELECT * FROM positions WHERE id=$1"#)
    //         .bind(id)
    //         .fetch_one(pool)
    //         .await?;
    //     Ok(position)
    // }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, r#"SELECT * FROM positions ORDER BY id DESC LIMIT 1"#)
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
}

#[allow(dead_code)]
pub async fn insert_list(pool: &PgPool, data: &[Position]) -> sqlx::Result<()> {
    let date: Vec<_> = data.iter().map(|d| d.date).collect();
    let latitude: Vec<_> = data.iter().map(|d| d.latitude).collect();
    let longitude: Vec<_> = data.iter().map(|d| d.longitude).collect();
    let speed: Vec<_> = data.iter().map(|d| d.speed).collect();
    let power: Vec<_> = data.iter().map(|d| d.power).collect();
    let odometer: Vec<_> = data.iter().map(|d| d.odometer).collect();
    let ideal_battery_range_km: Vec<_> = data.iter().map(|d| d.ideal_battery_range_km).collect();
    let battery_level: Vec<_> = data.iter().map(|d| d.battery_level).collect();
    let outside_temp: Vec<_> = data.iter().map(|d| d.outside_temp).collect();
    let elevation: Vec<_> = data.iter().map(|d| d.elevation).collect();
    let fan_status: Vec<_> = data.iter().map(|d| d.fan_status).collect();
    let driver_temp_setting: Vec<_> = data.iter().map(|d| d.driver_temp_setting).collect();
    let passenger_temp_setting: Vec<_> = data.iter().map(|d| d.passenger_temp_setting).collect();
    let is_climate_on: Vec<_> = data.iter().map(|d| d.is_climate_on).collect();
    let is_rear_defroster_on: Vec<_> = data.iter().map(|d| d.is_rear_defroster_on).collect();
    let is_front_defroster_on: Vec<_> = data.iter().map(|d| d.is_front_defroster_on).collect();
    let car_id: Vec<_> = data.iter().map(|d| d.car_id).collect();
    let drive_id: Vec<_> = data.iter().map(|d| d.drive_id).collect();
    let inside_temp: Vec<_> = data.iter().map(|d| d.inside_temp).collect();
    let battery_heater: Vec<_> = data.iter().map(|d| d.battery_heater).collect();
    let battery_heater_on: Vec<_> = data.iter().map(|d| d.battery_heater_on).collect();
    let battery_heater_no_power: Vec<_> = data.iter().map(|d| d.battery_heater_no_power).collect();
    let est_battery_range_km: Vec<_> = data.iter().map(|d| d.est_battery_range_km).collect();
    let rated_battery_range_km: Vec<_> = data.iter().map(|d| d.rated_battery_range_km).collect();
    let usable_battery_level: Vec<_> = data.iter().map(|d| d.usable_battery_level).collect();
    let tpms_pressure_fl: Vec<_> = data.iter().map(|d| d.tpms_pressure_fl).collect();
    let tpms_pressure_fr: Vec<_> = data.iter().map(|d| d.tpms_pressure_fr).collect();
    let tpms_pressure_rl: Vec<_> = data.iter().map(|d| d.tpms_pressure_rl).collect();
    let tpms_pressure_rr: Vec<_> = data.iter().map(|d| d.tpms_pressure_rr).collect();

    sqlx::query(
        r#"
        INSERT INTO positions
        (
            date,
            latitude,
            longitude,
            speed,
            power,
            odometer,
            ideal_battery_range_km,
            battery_level,
            outside_temp,
            elevation,
            fan_status,
            driver_temp_setting,
            passenger_temp_setting,
            is_climate_on,
            is_rear_defroster_on,
            is_front_defroster_on,
            car_id,
            drive_id,
            inside_temp,
            battery_heater,
            battery_heater_on,
            battery_heater_no_power,
            est_battery_range_km,
            rated_battery_range_km,
            usable_battery_level,
            tpms_pressure_fl,
            tpms_pressure_fr,
            tpms_pressure_rl,
            tpms_pressure_rr
        )
        SELECT * FROM UNNEST($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
            $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)"#,
    )
    .bind(&date)
    .bind(latitude)
    .bind(longitude)
    .bind(speed)
    .bind(power)
    .bind(odometer)
    .bind(ideal_battery_range_km)
    .bind(battery_level)
    .bind(outside_temp)
    .bind(elevation)
    .bind(fan_status)
    .bind(driver_temp_setting)
    .bind(passenger_temp_setting)
    .bind(is_climate_on)
    .bind(is_rear_defroster_on)
    .bind(is_front_defroster_on)
    .bind(car_id)
    .bind(drive_id)
    .bind(inside_temp)
    .bind(battery_heater)
    .bind(battery_heater_on)
    .bind(battery_heater_no_power)
    .bind(est_battery_range_km)
    .bind(rated_battery_range_km)
    .bind(usable_battery_level)
    .bind(tpms_pressure_fl)
    .bind(tpms_pressure_fr)
    .bind(tpms_pressure_rl)
    .bind(tpms_pressure_rr)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn get_latest(pool: &sqlx::Pool<sqlx::Postgres>, car_id: i16) -> sqlx::Result<Position> {
    sqlx::query_as!(
        Position,
        r#"
        SELECT * FROM positions WHERE car_id = $1 ORDER BY date DESC LIMIT 1
        "#,
        car_id
    )
    .fetch_one(pool)
    .await
}
