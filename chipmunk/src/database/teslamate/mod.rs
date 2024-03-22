use sqlx::PgPool;

mod charges;
mod charging_process;
mod drive;
mod position;

pub trait Teslamate {
    // required methods
    fn table_name() -> &'static str;

    // Optional methods
    #[allow(async_fn_in_trait)]
    async fn tm_num_rows(pool: &PgPool) -> sqlx::Result<i64> {
        let resp = sqlx::query(
            format!(r#"SELECT COUNT(*) as count FROM {}"#, Self::table_name()).as_str(),
        )
        .fetch_one(pool)
        .await?;
        Ok(sqlx::Row::get::<i64, _>(&resp, "count"))
    }

    fn tm_get_last(_pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<Self>> + Send
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_last` is not implemented for `{}` table!", Self::table_name()))
        }
    }

    fn tm_get_id(
        _pool: &PgPool,
        _id: i64,
    ) -> impl std::future::Future<Output = sqlx::Result<Self>> + Send
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_last` is not implemented for `{}` table!", Self::table_name()))
        }
    }

    fn tm_get_all(_pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<Vec<Self>>>
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_all` is not implemented for `{}` table!", Self::table_name()))
        }
    }
}
