use sqlx::PgPool;

pub mod tables;
pub mod types;
mod traits;

pub use traits::DBTable;

pub async fn initialize(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(url).await?;

    log::info!("Running database migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    tables::initialize(&pool).await?;

    Ok(pool)
}

pub async fn initilize_car_data(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(url).await?;

    Ok(pool)
}
