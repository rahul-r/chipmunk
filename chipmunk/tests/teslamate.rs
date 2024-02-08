use chipmunk::database::{tables::drive::Drive, Teslamate};


#[tokio::test]
async fn test_teslamate() {
    dotenvy::dotenv().ok();
    let url = std::env::var("TESLAMATE_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TESLAMATE_DATABASE_URL`");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let tm_drive = Drive::tm_get_last(&pool).await.unwrap();
    dbg!(tm_drive);
}