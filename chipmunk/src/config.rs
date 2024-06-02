use std::{
    env,
    marker::Send,
    sync::{atomic::AtomicU32, Arc, Mutex},
};

use anyhow::Context;
use tesla_api::auth::AuthResponse;
use tokio::sync::watch;

use crate::database::{
    tables::{settings::Settings, token::Token},
    DBTable,
};

#[allow(dead_code)]
#[derive(Clone)]
pub struct EnvVars {
    pub encryption_key: String,
    pub database_url: String,
    pub car_data_database_url: Option<String>,
    pub http_port: u16,
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

    Ok(EnvVars {
        encryption_key,
        database_url,
        car_data_database_url,
        http_port,
    })
}

type HandlerType<T> = Box<dyn Fn(T) + Send>;

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
    handlers: Arc<Mutex<Vec<HandlerType<T>>>>,
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
            handlers: Arc::new(Mutex::new(vec![])),
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

    pub fn subscribe(&mut self, handler: fn(T)) {
        match self.handlers.lock() {
            Ok(mut handlers) => handlers.push(Box::new(handler)),
            Err(e) => log::error!("{e}"),
        };
    }

    pub fn watch(&self) -> watch::Receiver<T> {
        //let (sender, receiver) = watch::channel(T::default());

        //match self.watchers.lock() {
        //    Ok(mut watchers) => watchers.push(receiver),
        //    Err(e) => log::error!("{e}"),
        //};
        self.watcher.subscribe()
    }

    // pub fn subscribe_closure<F>(&mut self, handler: F)
    // where
    //     F: Fn(T) + Send + 'static,
    // {
    //     match self.handlers.lock() {
    //         Ok(mut handlers) => handlers.push(Box::new(handler)),
    //         Err(e) => log::error!("{e}"),
    //     };
    // }

    //pub fn subscribe_async<F>(&mut self, handler: impl futures::Future<Output = ()> + Send)
    //where
    //    F: Fn(T) + Send + 'static,
    //{
    //    match self.handlers_async.lock() {
    //        Ok(mut handlers) => handlers.push(Box::new(handler)),
    //        Err(e) => log::error!("{e}"),
    //    };
    //}

    fn emit(&self) {
        match self.handlers.lock() {
            Ok(handlers) => {
                for handler in handlers.iter() {
                    handler(self.f.clone());
                }
            }
            Err(e) => log::error!("{e}"),
        }

        if let Err(e) = self.watcher.send(self.f.clone()) {
            log::error!("{e}");
        }
    }
}

#[derive(Clone)]
pub struct Config {
    pub logging_enabled: Arc<Mutex<Field<bool>>>,
    //pub logging_enabled: Arc<Field<Arc<AtomicBool>>>,
    pub test_flag: Arc<Field<Arc<AtomicU32>>>,
    pub logging_period_ms: Arc<Mutex<Field<i32>>>,
    pub access_token: Arc<Mutex<Field<String>>>,
    pub refresh_token: Arc<Mutex<Field<String>>>,
    pub encryption_key: Arc<Mutex<Field<String>>>,
    pub database_url: Arc<Mutex<Field<String>>>,
    pub car_data_database_url: Arc<Mutex<Field<Option<String>>>>,
    pub http_port: Arc<Mutex<Field<u16>>>,
}

impl Config {
    pub async fn new(pool: &sqlx::PgPool) -> Self {
        let env_vars = match load_env_vars() {
            Ok(v) => v,
            Err(e) => panic!("{e}"), // TODO: return Result/Err instead of panicking
        };

        let tokens = match Token::db_get_last(pool, &env_vars.encryption_key).await {
            Ok(t) => t,
            Err(e) => {
                log::error!("{e}");
                AuthResponse::default()
            }
        };

        let settings = match Settings::db_get_last(pool).await {
            Ok(settings) => settings,
            Err(e) => {
                log::error!("Error loading settings from database: {e}. Using default values");
                Settings::default()
            }
        };

        Self {
            logging_enabled: Arc::new(Mutex::new(Field::new(true))),
            test_flag: Arc::new(Field::new(Arc::new(AtomicU32::new(
                settings.logging_period_ms as u32,
            )))),
            logging_period_ms: Arc::new(Mutex::new(Field::new(settings.logging_period_ms))),
            access_token: Arc::new(Mutex::new(Field::new(tokens.access_token))),
            refresh_token: Arc::new(Mutex::new(Field::new(tokens.refresh_token))),
            encryption_key: Arc::new(Mutex::new(Field::new(env_vars.encryption_key))),
            database_url: Arc::new(Mutex::new(Field::new(env_vars.database_url))),
            car_data_database_url: Arc::new(Mutex::new(Field::new(env_vars.car_data_database_url))),
            http_port: Arc::new(Mutex::new(Field::new(env_vars.http_port))),
        }
    }
}
