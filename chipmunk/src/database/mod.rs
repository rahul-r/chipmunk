use sqlx::PgPool;

pub mod tables;
mod traits;
pub mod types;

pub use traits::DBTable;

pub mod teslamate;
pub use teslamate::Teslamate;

pub async fn initialize(url: &str) -> anyhow::Result<PgPool> {
    log::info!("Connecting to postgres database: {url}");
    let pool = PgPool::connect(url).await?;

    log::info!("Running database migrations");
    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        log::error!("Error running migrations: {e}");
    }

    tables::initialize(&pool).await?;

    Ok(pool)
}

pub async fn initialize_car_data(url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(url).await?;

    Ok(pool)
}
