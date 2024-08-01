use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::openstreetmap::{self, OsmResponse};

use super::DBTable;

#[derive(Debug, Default, Clone)]
pub struct Address {
    pub id: i64,
    pub display_name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub name: Option<String>,
    pub house_number: Option<String>,
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub city: Option<String>,
    pub county: Option<String>,
    pub postcode: Option<String>,
    pub state: Option<String>,
    pub state_district: Option<String>,
    pub country: Option<String>,
    pub raw: Option<serde_json::Value>,
    pub inserted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub osm_id: Option<i64>,
    pub osm_type: Option<String>,
}

impl Address {
    pub async fn from(latitude: f64, longitude: f64) -> anyhow::Result<Self> {
        let get_raw_osm = |osm: OsmResponse| match serde_json::to_value(osm) {
            Ok(r) => Some(r),
            Err(e) => {
                log::error!("Error converting OSM response struct to json: {e}");
                None
            }
        };

        let mut osm = OsmResponse::default();
        let mut raw_osm: Option<serde_json::Value> = None;
        match openstreetmap::osm_client() {
            Ok(client) => {
                match openstreetmap::reverse_geocode(&client, &latitude, &longitude).await {
                    Ok(a) => {
                        osm = a.clone();
                        raw_osm = get_raw_osm(a);
                    }
                    Err(e) => log::error!("Reverse geocoding error: {e}"),
                }
            }
            Err(e) => log::error!("Error creating openstreetmap client: {e}"),
        }

        Ok(Self {
            id: 0,
            display_name: osm.get_formatted_display_name(),
            latitude: Some(latitude),
            longitude: Some(longitude),
            name: osm.get_name(),
            house_number: osm.get_house_number(),
            road: osm.get_road(),
            neighbourhood: osm.get_neighbourhood(),
            city: osm.get_city(),
            county: osm.get_county(),
            postcode: osm.get_postcode(),
            state: osm.get_state(),
            state_district: osm.get_state_district(),
            country: osm.get_country(),
            raw: raw_osm,
            inserted_at: Utc::now(),
            updated_at: Utc::now(),
            osm_id: osm.osm_id,
            osm_type: osm.osm_type,
        })
    }

    pub async fn from_opt(latitude: Option<f64>, longitude: Option<f64>) -> anyhow::Result<Self> {
        if let (Some(lat), Some(lon)) = (latitude, longitude) {
            Self::from(lat, lon).await
        } else {
            anyhow::bail!(
                "Invalid latitude and/or longitude: ({:?}, {:?})",
                latitude,
                longitude
            );
        }
    }
}

impl DBTable for Address {
    fn table_name() -> &'static str {
        "addresses"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        // NOTE: Using the 'ON CONFLICT' will cause the id field to increment even if the insert is
        // skipped due to the conflict. This will cause missing IDs in the table.
        let id = sqlx::query!(
            r#"
        WITH e AS (
            INSERT INTO addresses
            (
                display_name,
                latitude,
                longitude,
                name,
                house_number,
                road,
                neighbourhood,
                city,
                county,
                postcode,
                state,
                state_district,
                country,
                raw,
                inserted_at,
                updated_at,
                osm_id,
                osm_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            ON CONFLICT(osm_id, osm_type) DO NOTHING
            RETURNING id
        )
        SELECT * FROM e
        UNION
            SELECT id FROM addresses WHERE osm_id=$17 AND osm_type=$18
        "#,
            self.display_name,
            self.latitude,
            self.longitude,
            self.name,
            self.house_number,
            self.road,
            self.neighbourhood,
            self.city,
            self.county,
            self.postcode,
            self.state,
            self.state_district,
            self.country,
            self.raw,
            self.inserted_at,
            self.updated_at,
            self.osm_id,
            self.osm_type,
        )
        .fetch_one(pool)
        .await?
        .id;

        if id.is_none() {
            log::error!("Unexpected row ID read from addresses table");
        }

        Ok(id.unwrap_or(0) as i64)
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, r#"SELECT * FROM addresses ORDER BY id DESC LIMIT 1"#)
            .fetch_one(pool)
            .await
    }
}
