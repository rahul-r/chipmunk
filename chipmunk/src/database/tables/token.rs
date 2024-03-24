use anyhow::Context;
use chrono::{Duration, DateTime, Utc};
use sqlx::PgPool;
use tesla_api::auth::AuthResponse;

use crate::{
    utils::crypto::{decrypt, encrypt},
    utils::seconds_remaining,
};

#[cfg(test)]
use crate::database::initialize;
#[cfg(test)]
use std::env;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Token {
    id: i32,
    access_token: Option<Vec<u8>>,
    access_token_iv: Option<Vec<u8>>,
    refresh_token: Option<Vec<u8>>,
    refresh_token_iv: Option<Vec<u8>>,
    id_token: Option<Vec<u8>>,
    id_token_iv: Option<Vec<u8>>,
    access_token_expires_at: DateTime<Utc>,
    token_type: Option<String>,
    updated_at: DateTime<Utc>,
}

impl Token {
    pub async fn exists(pool: &PgPool) -> anyhow::Result<bool> {
        let result = sqlx::query!(
                r#"SELECT EXISTS ( SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'tokens')"#
            )
            .fetch_one(pool)
            .await?
            .exists;

        match result.context("Cannot check token table existence in database")? {
            true => {
                // table exists, check if there are any keys in the table
                let num_rows = sqlx::query!(r#"SELECT COUNT(id) FROM tokens"#)
                    .fetch_one(pool)
                    .await?
                    .count
                    .expect("Cannot get number of rows in the database table");
                Ok(num_rows > 0)
            }
            _ => Ok(false),
        }
    }

    pub async fn db_insert(
        pool: &PgPool,
        tokens: AuthResponse,
        encryption_key: &str,
    ) -> anyhow::Result<()> {
        log::info!("Encrypting tokens");
        let (refresh_token, refresh_token_iv) = encrypt(&tokens.refresh_token, encryption_key)?;
        let (access_token, access_token_iv) = encrypt(&tokens.access_token, encryption_key)?;
        let (id_token, id_token_iv) = encrypt(&tokens.id_token, encryption_key)?;

        // Calculate the access token expiration time
        let time_now = Utc::now();
        let expires_at =
            time_now + Duration::try_seconds(tokens.expires_in as i64).unwrap_or_default();

        log::info!("Inserting tokens into database");
        sqlx::query!(
            r#"
            INSERT INTO tokens
            (
                refresh_token,
                refresh_token_iv,
                access_token,
                access_token_iv,
                access_token_expires_at,
                id_token,
                id_token_iv,
                token_type,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO UPDATE
                    SET
                        refresh_token = excluded.refresh_token,
                        refresh_token_iv = excluded.refresh_token_iv,
                        access_token = excluded.access_token,
                        access_token_iv = excluded.access_token_iv,
                        access_token_expires_at = excluded.access_token_expires_at,
                        id_token = excluded.id_token,
                        id_token_iv = excluded.id_token_iv,
                        token_type = excluded.token_type,
                        updated_at = excluded.updated_at
            "#,
            refresh_token,
            refresh_token_iv,
            access_token,
            access_token_iv,
            expires_at,
            id_token,
            id_token_iv,
            tokens.token_type,
            time_now
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn db_get_last(pool: &PgPool, encryption_key: &str) -> anyhow::Result<AuthResponse> {
        log::info!("Getting tokens from database");

        let token_table = sqlx::query_as!(Token, r#"SELECT * FROM tokens"#)
            .fetch_one(pool)
            .await?;

        log::info!("Decrypting tokens");

        fn decrypt_token(
            token: Option<Vec<u8>>,
            key: &str,
            iv: Option<Vec<u8>>,
        ) -> anyhow::Result<String> {
            let token_value = match token {
                Some(t) => t,
                None => anyhow::bail!("Received invalid token from database"),
            };

            let iv_value = match iv {
                Some(i) => i,
                None => anyhow::bail!("Received invalid iv from database"),
            };

            decrypt(&token_value, key, &iv_value)
        }

        let token = AuthResponse {
            refresh_token: decrypt_token(
                token_table.refresh_token,
                encryption_key,
                token_table.refresh_token_iv,
            )?,
            access_token: decrypt_token(
                token_table.access_token,
                encryption_key,
                token_table.access_token_iv,
            )?,
            id_token: decrypt_token(
                token_table.id_token,
                encryption_key,
                token_table.id_token_iv,
            )?,
            expires_in: seconds_remaining(token_table.access_token_expires_at),
            token_type: token_table.token_type.unwrap_or_else(|| {
                log::warn!("Received invalid token type from database");
                "".into()
            }),
        };

        Ok(token)
    }
}

#[tokio::test]
async fn test_key_retrieval() {
    dotenvy::dotenv().ok();
    let url = &env::var("TEST_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TEST_DATABASE_URL`");
    let pool = initialize(url).await.expect("Error initializing database");
    let encryption_key = "secret password acbdefghijklmnop";
    Token::db_get_last(&pool, encryption_key)
        .await
        .expect("Error getting tokens from database");
}

#[tokio::test]
async fn test_key_insertion() {
    dotenvy::dotenv().ok();
    let url = &env::var("TEST_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TEST_DATABASE_URL`");
    let pool = initialize(url).await.expect("Error initializing database");
    let encryption_key = "secret password acbdefghijklmnop";
    let tokens = AuthResponse {
        access_token: "access_token".into(),
        refresh_token: "refresh_token".into(),
        id_token: "id_token".into(),
        expires_in: 1234,
        token_type: "Bearer".into(),
    };
    Token::db_insert(&pool, tokens, encryption_key)
        .await
        .expect("Error inserting tokens to database");
}
