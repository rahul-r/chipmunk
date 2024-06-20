use std::str::FromStr;

use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfTemperature enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum TemperatureUnit {
    #[default]
    F,
    C,
}

impl FromStr for TemperatureUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "F" {
            Ok(Self::F)
        } else if s == "C" {
            Ok(Self::C)
        } else {
            Err("Invalid unit string `{s}`".into())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Temperature {
    pub fahrenheit: f32,
}

impl Temperature {
    pub fn from_celsius(celsius: f32) -> Self {
        Self {
            fahrenheit: celsius * 1.8 + 32.0,
        }
    }

    pub fn from_fahrenheit(fahrenheit: f32) -> Self {
        Self { fahrenheit }
    }

    pub fn as_celsius(&self) -> f32 {
        (self.fahrenheit - 32.0) * 0.556
    }

    pub fn as_fahrenheit(&self) -> f32 {
        self.fahrenheit
    }

    pub fn to_string(&self, unit: &TemperatureUnit) -> String {
        match unit {
            TemperatureUnit::F => format!("{} °F", self.as_fahrenheit()),
            TemperatureUnit::C => format!("{} °C", self.as_celsius()),
        }
    }
}
