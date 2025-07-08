use std::{
    env,
    marker::Send,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use tesla_api::auth::AuthResponse;
use tokio::sync::watch;

use crate::database::{
    tables::{settings::Settings, token::Token},
    types::{UnitOfLength, UnitOfPressure, UnitOfTemperature},
    DBTable,
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct EnvVars {
    pub encryption_key: String,
    pub database_url: String,
    pub car_data_database_url: Option<String>,
    pub http_port: u16,
    pub http_root: Option<String>,
}

pub fn load_env_vars() -> anyhow::Result<EnvVars> {
    let encryption_key =
        env::var("TOKEN_ENCRYPTION_KEY").context("Please provide TOKEN_ENCRYPTION_KEY")?;
    let database_url = env::var("DATABASE_URL").context("Please provide DATABASE_URL")?;
    let car_data_database_url = match env::var("CAR_DATA_DATABASE_URL") {
        Ok(v) => Some(v),
        Err(e) => match e {
            env::VarError::NotPresent => None,
            env::VarError::NotUnicode(e) => {
                anyhow::bail!(
                    "Invalid value for environment variable CAR_DATA_DATABASE_URL: {e:?}"
                );
            }
        },
    };

    const DEFAULT_PORT: u16 = 3072;
    let http_port = match env::var("HTTP_PORT") {
        Ok(port) => port.parse().unwrap_or_else(|e| {
            log::error!("Invalid HTTP_PORT `{port}`: {e}");
            log::info!("Using default port {DEFAULT_PORT}");
            DEFAULT_PORT
        }),
        Err(e) => {
            log::warn!("Error reading HTTP_PORT from environment: {e}");
            log::info!("Using default port {DEFAULT_PORT}");
            DEFAULT_PORT
        }
    };

    let http_root = env::var("HTTP_ROOT").ok();

    Ok(EnvVars {
        encryption_key,
        database_url,
        car_data_database_url,
        http_port,
        http_root,
    })
}

#[macro_export]
macro_rules! set_config {
    ($config_param:expr, $value:expr) => {{
        match $config_param.lock().map(|mut lock| lock.set($value)) {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!(
                    "Error getting lock on mutex `{}`: {e}",
                    stringify!($config_param)
                );
                Err(e)
            }
        }
        .ok()
    }};
}

#[macro_export]
macro_rules! get_config {
    ($config_param:expr) => {{
        //$arc_mutex.lock().map(|lock| lock.get())
        match $config_param.lock() {
            Ok(v) => Ok(v),
            Err(e) => {
                log::error!(
                    "Error getting lock on mutex `{}`: {e}",
                    stringify!($config_param)
                );
                Err(e)
            }
        }
        .map(|lock| lock.get())
    }};
}

pub struct Field<T> {
    f: T,
    watcher: watch::Sender<T>,
    _receiver: watch::Receiver<T>,
}

impl<T> Field<T>
where
    T: Clone + Default + 'static + Sync + Send,
{
    pub fn new(value: T) -> Self {
        let w = watch::channel(T::default());
        Self {
            f: value,
            watcher: w.0,
            _receiver: w.1,
        }
    }

    pub fn get(&self) -> T {
        self.f.clone()
    }

    pub fn set(&mut self, value: T) {
        self.f = value;
        self.emit();
    }

    pub fn watch(&self) -> watch::Receiver<T> {
        self.watcher.subscribe()
    }

    fn emit(&self) {
        if let Err(e) = self.watcher.send(self.f.clone()) {
            log::error!("{e}");
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub logging_enabled: Arc<Mutex<Field<bool>>>,
    //pub logging_enabled: Arc<Field<Arc<AtomicBool>>>,
    pub logging_period_ms: Arc<Mutex<Field<i32>>>,
    pub access_token: Arc<Mutex<Field<String>>>,
    pub refresh_token: Arc<Mutex<Field<String>>>,
    pub encryption_key: Arc<Mutex<Field<String>>>,
    pub database_url: Arc<Mutex<Field<String>>>,
    pub car_data_database_url: Arc<Mutex<Field<Option<String>>>>,
    pub http_port: Arc<Mutex<Field<u16>>>,
    pub http_root: Arc<Mutex<Field<Option<String>>>>,
    pub unit_of_length: Arc<Mutex<Field<UnitOfLength>>>,
    pub unit_of_temperature: Arc<Mutex<Field<UnitOfTemperature>>>,
    pub unit_of_pressure: Arc<Mutex<Field<UnitOfPressure>>>,
}

impl Config {
    pub async fn new(pool: &sqlx::PgPool) -> Self {
        let env_vars = match load_env_vars() {
            Ok(v) => v,
            Err(e) => panic!("{e}"), // TODO: return Result/Err instead of panicking
        };

        let tokens = Token::db_get_last(pool, &env_vars.encryption_key)
            .await
            .unwrap_or_else(|e| {
                log::error!("{e}");
                AuthResponse::default()
            });
        
        let settings = Settings::db_get_last(pool).await.unwrap_or_else(|e| {
            log::error!("Error loading settings from database: {e}. Using default values");
            Settings::default()
        });

        Self {
            logging_enabled: Arc::new(Mutex::new(Field::new(true))),
            logging_period_ms: Arc::new(Mutex::new(Field::new(settings.logging_period_ms))),
            access_token: Arc::new(Mutex::new(Field::new(tokens.access_token))),
            refresh_token: Arc::new(Mutex::new(Field::new(tokens.refresh_token))),
            encryption_key: Arc::new(Mutex::new(Field::new(env_vars.encryption_key))),
            database_url: Arc::new(Mutex::new(Field::new(env_vars.database_url))),
            car_data_database_url: Arc::new(Mutex::new(Field::new(env_vars.car_data_database_url))),
            http_port: Arc::new(Mutex::new(Field::new(env_vars.http_port))),
            http_root: Arc::new(Mutex::new(Field::new(env_vars.http_root))),
            unit_of_length: Arc::new(Mutex::new(Field::new(settings.unit_of_length))),
            unit_of_temperature: Arc::new(Mutex::new(Field::new(settings.unit_of_temperature))),
            unit_of_pressure: Arc::new(Mutex::new(Field::new(settings.unit_of_pressure))),
        }
    }
}
