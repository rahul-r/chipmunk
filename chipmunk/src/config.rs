use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Clone)]
pub enum ConfigItem {
    #[default]
    None,
    AccessToken(String),
    RefreshToken(String),
    LoggingPeriodMs(i32),
}
use ConfigItem as ci;

impl ConfigItem {
    pub fn name(&self) -> &str {
        match self {
            Self::AccessToken(_) => "AccessToken",
            Self::RefreshToken(_) => "RefreshToken",
            Self::LoggingPeriodMs(_) => "LoggingPeriodMs",
            Self::None => "None",
        }
    }
}

struct Fields {
    pub access_token: String,
    pub refresh_token: String,
    pub logging_period_ms: i32,
}

type HandlerType = Box<dyn Fn(ConfigItem) + Send>;

#[derive(Clone)]
pub struct Config {
    fields: Arc<Mutex<Fields>>,
    handlers: Arc<Mutex<HashMap<String, Vec<HandlerType>>>>,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Config {
            fields: Arc::new(Mutex::new(Fields {
                access_token: "".to_string(),
                refresh_token: "".to_string(),
                logging_period_ms: 1000,
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
        }

        self.emit(item);
    }
}
