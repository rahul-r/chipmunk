use crate::utils::{
    avg_option, max_option, min_option, sub_option, time_diff_minutes_i16,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::{position::Position, DBTable};

#[derive(Debug, Default, Clone, sqlx::FromRow)]
pub struct Drive {
    pub id: i32,
    pub in_progress: bool, // This is used to track the current status of driving
    pub start_date: DateTime<Utc>,
    pub end_date: Option<DateTime<Utc>>,
    pub outside_temp_avg: Option<f32>,
    pub speed_max: Option<f32>,
    pub power_max: Option<f32>,
    pub power_min: Option<f32>,
    pub start_ideal_range_km: Option<f32>,
    pub end_ideal_range_km: Option<f32>,
    pub start_km: Option<f32>,
    pub end_km: Option<f32>,
    pub distance: Option<f32>,
    pub duration_min: Option<i16>,
    pub car_id: i16,
    pub inside_temp_avg: Option<f32>,
    pub start_address_id: Option<i32>,
    pub end_address_id: Option<i32>,
    pub start_rated_range_km: Option<f32>,
    pub end_rated_range_km: Option<f32>,
    pub start_position_id: Option<i32>,
    pub end_position_id: Option<i32>,
    pub start_geofence_id: Option<i32>,
    pub end_geofence_id: Option<i32>,
}

impl Drive {
    pub fn start(
        position: &Position,
        car_foreign_key: i16,
        start_address_id: Option<i32>,
        start_geofence_id: Option<i32>,
    ) -> Self {
        Self {
            in_progress: true,
            car_id: car_foreign_key,
            start_date: position.date.unwrap_or_else(|| {
                log::warn!("Position date is None, using current system time");
                chrono::Utc::now()
            }),
            start_ideal_range_km: position.ideal_battery_range_km,
            start_km: position.odometer,
            end_ideal_range_km: position.ideal_battery_range_km,
            start_address_id,
            start_rated_range_km: position.rated_battery_range_km,
            end_rated_range_km: position.rated_battery_range_km,
            start_position_id: position.id,
            end_position_id: position.id,
            start_geofence_id,
            inside_temp_avg: position.inside_temp,
            outside_temp_avg: position.outside_temp,
            speed_max: position.speed,
            power_max: position.power,
            power_min: position.power,
            end_km: position.odometer,
            distance: Some(0.0),
            duration_min: Some(0),
            ..Self::default()
        }
    }

    pub fn reset(&self) -> Self {
        Self {
            id: self.id,
            ..Self::default()
        }
    }

    pub fn stop(
        &self,
        position: &Position,
        end_address_id: Option<i32>,
        end_geofence_id: Option<i32>,
    ) -> Self {
        Self {
            in_progress: false,
            end_date: position.date,
            end_position_id: position.id,
            end_address_id,
            end_geofence_id,
            ..self.update(position)
        }
    }

    pub fn update(&self, position: &Position) -> Self {
        Self {
            inside_temp_avg: avg_option(self.inside_temp_avg, position.inside_temp),
            outside_temp_avg: avg_option(self.outside_temp_avg, position.outside_temp),
            speed_max: max_option(self.speed_max, position.speed).map(|v| v.floor()), // the .floor() is to make the values compatible with teslamate
            power_min: min_option(self.power_min, position.power).map(|v| v.floor()), // the .floor() is to make the values compatible with teslamate
            power_max: max_option(self.power_max, position.power).map(|v| v.floor()), // the .floor() is to make the values compatible with teslamate
            end_ideal_range_km: position.ideal_battery_range_km,
            end_km: position.odometer,
            distance: sub_option(position.odometer, self.start_km),
            duration_min: time_diff_minutes_i16(Some(self.start_date), position.date),
            end_rated_range_km: position.rated_battery_range_km,
            end_position_id: position.id,
            ..self.clone()
        }
    }
}

impl DBTable for Drive {
    fn table_name() -> &'static str {
        "drives"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO drives
        (
            start_date,
            end_date,
            outside_temp_avg,
            speed_max,
            power_max,
            power_min,
            start_ideal_range_km,
            end_ideal_range_km,
            start_km,
            end_km,
            distance,
            duration_min,
            car_id,
            inside_temp_avg,
            start_address_id,
            end_address_id,
            start_rated_range_km,
            end_rated_range_km,
            start_position_id,
            end_position_id,
            start_geofence_id,
            end_geofence_id,
            in_progress
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
            $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
        RETURNING id"#,
            self.start_date,
            self.end_date,
            self.outside_temp_avg,
            self.speed_max,
            self.power_max,
            self.power_min,
            self.start_ideal_range_km,
            self.end_ideal_range_km,
            self.start_km,
            self.end_km,
            self.distance,
            self.duration_min,
            self.car_id,
            self.inside_temp_avg,
            self.start_address_id,
            self.end_address_id,
            self.start_rated_range_km,
            self.end_rated_range_km,
            self.start_position_id,
            self.end_position_id,
            self.start_geofence_id,
            self.end_geofence_id,
            self.in_progress,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }

    async fn db_update(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
        UPDATE drives
        SET
            start_date = $1,
            end_date = $2,
            outside_temp_avg = $3,
            speed_max = $4,
            power_max = $5,
            power_min = $6,
            start_ideal_range_km = $7,
            end_ideal_range_km = $8,
            start_km = $9,
            end_km = $10,
            distance = $11,
            duration_min = $12,
            inside_temp_avg = $13,
            start_address_id = $14,
            end_address_id = $15,
            start_rated_range_km = $16,
            end_rated_range_km = $17,
            start_position_id = $18,
            end_position_id = $19,
            start_geofence_id = $20,
            end_geofence_id = $21,
            in_progress = $22
        WHERE id = $23
        "#,
            self.start_date,
            self.end_date,
            self.outside_temp_avg,
            self.speed_max,
            self.power_max,
            self.power_min,
            self.start_ideal_range_km,
            self.end_ideal_range_km,
            self.start_km,
            self.end_km,
            self.distance,
            self.duration_min,
            self.inside_temp_avg,
            self.start_address_id,
            self.end_address_id,
            self.start_rated_range_km,
            self.end_rated_range_km,
            self.start_position_id,
            self.end_position_id,
            self.start_geofence_id,
            self.end_geofence_id,
            self.in_progress,
            self.id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        let cp = sqlx::query_as!(
            Self,
            r#"
                SELECT
                    id,
                    start_date,
                    end_date,
                    outside_temp_avg,
                    speed_max,
                    power_max,
                    power_min,
                    start_ideal_range_km,
                    end_ideal_range_km,
                    start_km,
                    end_km,
                    distance,
                    duration_min,
                    car_id,
                    inside_temp_avg,
                    start_address_id,
                    end_address_id,
                    start_rated_range_km,
                    end_rated_range_km,
                    start_position_id,
                    end_position_id,
                    start_geofence_id,
                    end_geofence_id,
                    in_progress
                FROM drives
                ORDER BY start_date DESC LIMIT 1
            "#
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
                    in_progress,
                    start_date,
                    end_date,
                    outside_temp_avg,
                    speed_max,
                    power_max,
                    power_min,
                    start_ideal_range_km,
                    end_ideal_range_km,
                    start_km,
                    end_km,
                    distance,
                    duration_min,
                    car_id,
                    inside_temp_avg,
                    start_address_id,
                    end_address_id,
                    start_rated_range_km,
                    end_rated_range_km,
                    start_position_id,
                    end_position_id,
                    start_geofence_id,
                    end_geofence_id
                FROM drives
                ORDER BY id ASC
            "#
        )
        .fetch_all(pool)
        .await
    }

    // async fn db_update_last(&self, pool: &PgPool) -> sqlx::Result<()> {
    //     // Get id of the latest drive
    //     let current_drive = sqlx::query!(
    //         r#"
    //     SELECT id,
    //         outside_temp_avg,
    //         speed_max,
    //         power_max,
    //         power_min,
    //         inside_temp_avg
    //     FROM drives
    //     ORDER BY id DESC LIMIT 1
    //     "#
    //     )
    //     .fetch_one(pool)
    //     .await?;
    //     let id = current_drive.id;

    //     // Update the latest drive
    //     sqlx::query!(
    //         r#"
    //     UPDATE drives
    //     SET
    //         end_date = $1,
    //         outside_temp_avg = (outside_temp_avg + $2) / 2,
    //         speed_max = GREATEST(speed_max, $3),
    //         power_max = GREATEST(power_max, $4),
    //         power_min = LEAST(power_min, $5),
    //         end_ideal_range_km = $6,
    //         end_km = $7,
    //         distance = COALESCE($8 - start_km, distance),
    //         duration_min = COALESCE(EXTRACT(EPOCH FROM ($9 - start_date)), duration_min),
    //         inside_temp_avg = (inside_temp_avg + $10) / 2,
    //         end_rated_range_km = $11,
    //         end_position_id = $12,
    //         status = $13
    //     WHERE id = $14
    //     "#,
    //         position.date,
    //         position.outside_temp,
    //         position.speed,
    //         position.power,
    //         position.power,
    //         position.ideal_battery_range_km,
    //         position.odometer,
    //         position.odometer,
    //         position.date,
    //         position.inside_temp,
    //         position.rated_battery_range_km,
    //         position.id,
    //         //drive.end_address_id,
    //         //drive.end_geofence_id,
    //         drive_status as DriveStatus,
    //         id,
    //     )
    //     .execute(pool)
    //     .await?;

    //     Ok(())
    // }
}
