use sqlx::PgPool;

pub trait DBTable {
    // required methods
    fn table_name() -> &'static str;
    #[allow(async_fn_in_trait)]
    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64>;

    // Optional methods
    #[allow(async_fn_in_trait)]
    async fn db_num_rows(pool: &PgPool) -> sqlx::Result<i64> {
        let resp = sqlx::query(
            format!(r#"SELECT COUNT(*) as count FROM {}"#, Self::table_name()).as_str(),
        )
        .fetch_one(pool)
        .await?;
        Ok(sqlx::Row::get::<i64, _>(&resp, "count"))
    }

    fn db_update(&self, _pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<()>> {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_update` is not implemented for `{}` table!", Self::table_name()))
        }
    }
    fn db_update_last(&self, _pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<()>>
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_update_last` is not implemented for `{}` table!", Self::table_name()))
        }
    }
    fn db_get_last(_pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<Self>>
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_last` is not implemented for `{}` table!", Self::table_name()))
        }
    }

    fn db_get_id(_pool: &PgPool, _id: i64) -> impl std::future::Future<Output = sqlx::Result<Self>>
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_id` is not implemented for `{}` table!", Self::table_name()))
        }
    }
    fn db_get_all(_pool: &PgPool) -> impl std::future::Future<Output = sqlx::Result<Vec<Self>>>
    where
        Self: Sized,
    {
        async {
            #[rustfmt::skip]
            panic!("{}", format!("`db_get_all` is not implemented for `{}` table!", Self::table_name()))
        }
    }
}
