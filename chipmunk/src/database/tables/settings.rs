use chrono::{NaiveDateTime, Utc};
use sqlx::PgPool;

use super::types::{Range, UnitOfLength, UnitOfPressure, UnitOfTemperature};

#[derive(Debug, Clone)]
pub struct Settings {
    #[allow(dead_code)]
    id: i32,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub unit_of_length: UnitOfLength,
    pub unit_of_temperature: UnitOfTemperature,
    pub preferred_range: Range,
    pub base_url: Option<String>,
    pub grafana_url: Option<String>,
    pub language: String, //DEFAULT 'en'::text,
    pub unit_of_pressure: UnitOfPressure,
    pub logging_period_ms: i32,
    pub log_at_startup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: 0,
            inserted_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            unit_of_length: UnitOfLength::default(),
            unit_of_temperature: UnitOfTemperature::default(),
            preferred_range: Range::default(),
            base_url: None,
            grafana_url: None,
            language: "en".into(),
            unit_of_pressure: UnitOfPressure::default(),
            logging_period_ms: 1500,
            log_at_startup: true,
        }
    }
}

impl Settings {
    pub async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO settings
        (
            inserted_at,
            updated_at,
            unit_of_length,
            unit_of_temperature,
            preferred_range,
            base_url,
            grafana_url,
            language,
            unit_of_pressure,
            logging_period_ms,
            log_at_startup
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (id) DO UPDATE
                SET
                    inserted_at = excluded.inserted_at,
                    updated_at = excluded.updated_at,
                    unit_of_length = excluded.unit_of_length,
                    unit_of_temperature = excluded.unit_of_temperature,
                    preferred_range = excluded.preferred_range,
                    base_url = excluded.base_url,
                    grafana_url = excluded.grafana_url,
                    language = excluded.language,
                    unit_of_pressure = excluded.unit_of_pressure,
                    logging_period_ms = excluded.logging_period_ms,
                    log_at_startup = excluded.log_at_startup
            RETURNING id"#,
            self.inserted_at,
            self.updated_at,
            self.unit_of_length as UnitOfLength,
            self.unit_of_temperature as UnitOfTemperature,
            self.preferred_range as Range,
            self.base_url,
            self.grafana_url,
            self.language,
            self.unit_of_pressure as UnitOfPressure,
            self.logging_period_ms,
            self.log_at_startup,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id)
    }

    pub async fn db_get(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as_unchecked!(Self, r#"SELECT * FROM settings"#)
            .fetch_one(pool)
            .await
    }
}

/// Initializes the settings in the database.
///
/// This function checks if there are any settings in the database by counting the number of rows in the `settings` table.
/// If the count is 0, it inserts the default settings into the database.
///
/// # Arguments
///
/// * `pool` - A reference to the database connection pool.
///
/// # Returns
///
/// * `anyhow::Result<()>` - Returns `Ok(())` if the settings were successfully initialized, or an `anyhow::Error` if an error occurred.
///
/// # Errors
///
/// This function will return an error if:
/// * There was a problem executing the SQL query.
/// * There was a problem inserting the default settings into the database.
/// * The count of settings could not be retrieved from the database.
pub(crate) async fn initialize(pool: &PgPool) -> anyhow::Result<()> {
    let count = sqlx::query!(r#"SELECT COUNT(id) FROM settings"#)
        .fetch_one(pool)
        .await?
        .count;
    if let Some(count) = count {
        if count == 0 {
            Settings::default().db_insert(pool).await?;
        }
    } else {
        anyhow::bail!("Error getting settings count");
    }
    Ok(())
}
