use sqlx::PgPool;

pub mod tables;
pub mod token;

pub async fn initialize(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(url).await?;

    log::info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap_or_else(print_err_and_exit!());

    tables::initialize(&pool).await?;

    Ok(pool)
}

pub async fn initilize_car_data(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(url).await?;

    Ok(pool)
}
