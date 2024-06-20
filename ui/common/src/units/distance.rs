use std::str::FromStr;

use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfLength enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum DistanceUnit {
    Mi,
    #[default]
    Km,
}

impl FromStr for DistanceUnit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "km" || s == "Km" {
            Ok(Self::Km)
        } else if s == "mi" || s == "Mi" {
            Ok(Self::Mi)
        } else {
            Err("Invalid unit string `{s}`".into())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Distance {
    pub miles: f64,
}

impl Distance {
    pub fn from_km(km: f64) -> Self {
        Self {
            miles: km / 1.609344,
        }
    }

    pub fn from_miles(miles: f64) -> Self {
        Self { miles }
    }

    pub fn as_km(&self) -> f64 {
        self.miles * 1.609344
    }

    pub fn as_miles(&self) -> f64 {
        self.miles
    }

    pub fn to_string(&self, unit: &DistanceUnit) -> String {
        match unit {
            DistanceUnit::Mi => format!("{} mi", self.as_miles().round()),
            DistanceUnit::Km => format!("{:.1} km", self.as_km()),
        }
    }
}
