use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ConfigItem {
    #[default]
    None,
    AccessToken(String),
    RefreshToken(String),
    LoggingPeriodMs(i32),
    LoggingEnabled(bool),
}
use tesla_api::auth::AuthResponse;
use ConfigItem as ci;

use crate::{
    database::{
        tables::{settings::Settings, token::Token},
        DBTable,
    },
    EnvVars,
};

impl ConfigItem {
    pub fn name(&self) -> &str {
        match self {
            Self::None => "None",
            Self::AccessToken(_) => "AccessToken",
            Self::RefreshToken(_) => "RefreshToken",
            Self::LoggingPeriodMs(_) => "LoggingPeriodMs",
            Self::LoggingEnabled(_) => "LoggingEnabled",
        }
    }

    pub fn get_bool(&self) -> bool {
        match &self {
            ci::LoggingEnabled(v) => *v,
            _ => panic!("{self:?} is not of type bool"),
        }
    }

    pub fn get_i32(&self) -> i32 {
        match &self {
            ci::LoggingPeriodMs(v) => *v,
            _ => panic!("{self:?} is not of type i32"),
        }
    }

    pub fn get_string(&self) -> String {
        match self {
            ci::AccessToken(v) => v.clone(),
            ci::RefreshToken(v) => v.clone(),
            _ => panic!("{self:?} is not of type String"),
        }
    }
}

struct Fields {
    pub access_token: String,
    pub refresh_token: String,
    pub logging_period_ms: i32,
    pub logging_enabled: bool,
}

type HandlerType = Box<dyn Fn(ConfigItem) + Send>;

#[derive(Clone)]
pub struct Config {
    fields: Arc<Mutex<Fields>>,
    handlers: Arc<Mutex<HashMap<String, Vec<HandlerType>>>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            fields: Arc::new(Mutex::new(Fields {
                access_token: "".to_string(),
                refresh_token: "".to_string(),
                logging_period_ms: 1000,
                logging_enabled: true,
            })),
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Config {
    pub async fn new(env: &EnvVars, pool: &sqlx::PgPool) -> Self {
        let tokens = match Token::db_get_last(pool, &env.encryption_key).await {
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

        Config {
            fields: Arc::new(Mutex::new(Fields {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                logging_period_ms: settings.logging_period_ms,
                logging_enabled: true,
            })),
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&mut self, event: ConfigItem, handler: HandlerType) {
        match self.handlers.lock() {
            Ok(mut handlers) => {
                let event_handlers = handlers.entry(event.name().to_string()).or_default();
                event_handlers.push(Box::new(handler));
            }
            Err(e) => log::error!("{e}"),
        };
    }

    fn emit(&self, event: ConfigItem) {
        match self.handlers.lock() {
            Ok(all_handlers) => {
                if let Some(handlers_of_event) = all_handlers.get(event.name()) {
                    for handler in handlers_of_event {
                        handler(event.clone());
                    }
                }
            }
            Err(e) => log::error!("{e}"),
        }
    }

    pub fn get(&self, item: &ConfigItem) -> ConfigItem {
        let configs = match self.fields.lock() {
            Ok(i) => i,
            Err(e) => {
                log::error!("Cannot acquire lock on config fields: {e}");
                return ConfigItem::None;
            }
        };

        match item {
            ci::None => ConfigItem::None,
            ci::AccessToken(_) => ConfigItem::AccessToken(configs.access_token.clone()),
            ci::RefreshToken(_) => ConfigItem::RefreshToken(configs.refresh_token.clone()),
            ci::LoggingPeriodMs(_) => ConfigItem::LoggingPeriodMs(configs.logging_period_ms),
            ci::LoggingEnabled(_) => ConfigItem::LoggingEnabled(configs.logging_enabled),
        }
    }

    pub fn set(&mut self, item: ConfigItem) {
        let mut configs = match self.fields.lock() {
            Ok(i) => i,
            Err(e) => {
                log::error!("Cannot acquire lock on config fields: {e}");
                return;
            }
        };

        match item.clone() {
            ci::None => (),
            ci::AccessToken(v) => configs.access_token = v,
            ci::RefreshToken(v) => configs.refresh_token = v,
            ci::LoggingPeriodMs(v) => configs.logging_period_ms = v,
            ci::LoggingEnabled(v) => configs.logging_enabled = v,
        }

        self.emit(item);
    }
}
