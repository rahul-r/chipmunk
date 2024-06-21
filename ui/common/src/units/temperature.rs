use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfTemperature enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum TemperatureUnit {
    #[default]
    F,
    C,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Temperature {
    pub celsius: f32,
}

impl Temperature {
    pub fn from_celsius(celsius: f32) -> Self {
        Self { celsius }
    }

    pub fn from_fahrenheit(fahrenheit: f32) -> Self {
        Self {
            celsius: (fahrenheit - 32.0) / 1.8,
        }
    }

    pub fn as_celsius(&self) -> f32 {
        self.celsius
    }

    pub fn as_fahrenheit(&self) -> f32 {
        self.celsius * 1.8 + 32.0
    }

    pub fn to_string(&self, unit: &TemperatureUnit) -> String {
        match unit {
            TemperatureUnit::F => format!("{:.0} °F", self.as_fahrenheit()),
            TemperatureUnit::C => format!("{:.1} °C", self.as_celsius()),
        }
    }
}
