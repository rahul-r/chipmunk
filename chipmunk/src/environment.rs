use std::env;

use anyhow::Context;

pub struct Environment {
    pub encryption_key: String,
    pub database_url: String,
    pub http_port: u16,
}

pub fn load() -> anyhow::Result<Environment> {
    let encryption_key =
        env::var("TOKEN_ENCRYPTION_KEY").context("Please provide TOKEN_ENCRYPTION_KEY")?;
    let database_url = env::var("DATABASE_URL").context("Please provide DATABASE_URL")?;

    const DEFAULT_PORT: u16 = 3072;
    let http_port = match env::var("HTTP_PORT") {
        Ok(port) => match port.parse() {
            Ok(p) => p,
            Err(e) => {
                log::error!("Invalid HTTP_PORT `{port}`: {e}");
                log::info!("Using default port {DEFAULT_PORT}");
                DEFAULT_PORT
            }
        },
        Err(e) => {
            log::warn!("Error reading HTTP_PORT from environment: {e}");
            log::info!("Using default port {DEFAULT_PORT}");
            DEFAULT_PORT
        }
    };

    Ok(Environment {
        encryption_key,
        database_url,
        http_port,
    })
}
