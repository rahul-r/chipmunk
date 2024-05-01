use sqlx::PgPool;

use crate::database::DBTable;

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

impl DBTable for CarSettings {
    fn table_name() -> &'static str {
        "car_settings"
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
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

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(Self, r#"SELECT * FROM car_settings ORDER BY id DESC LIMIT 1"#)
        .fetch_one(pool)
        .await
    }

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
