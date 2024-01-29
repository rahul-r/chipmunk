use chrono::{NaiveDateTime, Utc};
use sqlx::PgPool;

use super::{types::BillingType, DBTable};

#[derive(Debug)]
pub struct Geofence {
    pub name: String,
    pub latitude: f32,
    pub longitude: f32,
    pub radius: i16,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub cost_per_unit: Option<f32>,
    pub session_fee: Option<f32>,
    pub billing_type: BillingType,
}

impl Default for Geofence {
    fn default() -> Self {
        Self {
            name: "".into(),
            latitude: 0.0,
            longitude: 0.0,
            radius: 0,
            inserted_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            cost_per_unit: None,
            session_fee: None,
            billing_type: BillingType::default(),
        }
    }
}

impl DBTable for Geofence {
    fn table_name() -> &'static str {
        "geofences"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO geofences
        (
            name,
            latitude,
            longitude,
            radius,
            inserted_at,
            updated_at,
            cost_per_unit,
            session_fee,
            billing_type
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id"#,
            self.name,
            self.latitude,
            self.longitude,
            self.radius,
            self.inserted_at,
            self.updated_at,
            self.cost_per_unit,
            self.session_fee,
            self.billing_type as BillingType,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }
}

// pub async fn apply_geofence(pool: &PgPool, lat: f32, lon: f32, radius: f32) -> anyhow::Result<()> {
//     let except_id = -1; // TODO: find value of this from teslamate source code

//     // Set start_geofence_id in drives table
//     sqlx::query!(
//         r#"
//         UPDATE drives m
//         SET start_geofence_id = (
//           SELECT id
//           FROM geofences g
//           WHERE
//             earth_box(ll_to_earth(g.latitude, g.longitude), g.radius) @> ll_to_earth(p.latitude, p.longitude) AND
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) < g.radius AND
//             g.id != $4
//           ORDER BY
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) ASC
//           LIMIT 1
//         )
//         FROM positions p
//         WHERE
//           m.start_position_id = p.id AND
//           earth_box(ll_to_earth($1::real, $2::real), $3::real) @> ll_to_earth(p.latitude, p.longitude) AND
//           earth_distance(ll_to_earth($1, $2), ll_to_earth(latitude, p.longitude)) < $3
//         "#, lat, lon, radius, except_id
//     ).execute(pool).await?;

//     // Set end_geofence_id in drives table
//     sqlx::query!(
//         r#"
//         UPDATE drives m
//         SET end_geofence_id = (
//           SELECT id
//           FROM geofences g
//           WHERE
//             earth_box(ll_to_earth(g.latitude, g.longitude), g.radius) @> ll_to_earth(p.latitude, p.longitude) AND
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) < g.radius AND
//             g.id != $4
//           ORDER BY
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) ASC
//           LIMIT 1
//         )
//         FROM positions p
//         WHERE
//           m.end_position_id = p.id AND
//           earth_box(ll_to_earth($1::real, $2::real), $3::real) @> ll_to_earth(p.latitude, p.longitude) AND
//           earth_distance(ll_to_earth($1, $2), ll_to_earth(latitude, p.longitude)) < $3
//         "#, lat, lon, radius, except_id
//     ).execute(pool).await?;

//     // Set geofence_id in charging_processes table
//     sqlx::query!(
//         r#"
//         UPDATE charging_processes m
//         SET geofence_id = (
//           SELECT id
//           FROM geofences g
//           WHERE
//             earth_box(ll_to_earth(g.latitude, g.longitude), g.radius) @> ll_to_earth(p.latitude, p.longitude) AND
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) < g.radius AND
//             g.id != $4
//           ORDER BY
//             earth_distance(ll_to_earth(g.latitude, g.longitude), ll_to_earth(latitude, p.longitude)) ASC
//           LIMIT 1
//         )
//         FROM positions p
//         WHERE
//           m.position_id = p.id AND
//           earth_box(ll_to_earth($1::real, $2::real), $3::real) @> ll_to_earth(p.latitude, p.longitude) AND
//           earth_distance(ll_to_earth($1, $2), ll_to_earth(latitude, p.longitude)) < $3
//         "#, lat, lon, radius, except_id
//     ).execute(pool).await?;

//     Ok(())
// }
