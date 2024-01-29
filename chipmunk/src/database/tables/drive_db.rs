use super::{drive::Drive, DBTable};
use crate::database::tables::drive::DriveStatus;
use sqlx::PgPool;

impl DBTable for Drive {
    fn table_name() -> &'static str {
        "drives"
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
                    status AS "status!: DriveStatus"
                FROM drives
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
            status
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
            self.status.clone() as DriveStatus,
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
            status = $22
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
            self.status.clone() as DriveStatus,
            self.id,
        )
        .execute(pool)
        .await?;
        Ok(())
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
