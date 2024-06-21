use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfLength enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum DistanceUnit {
    #[default]
    Mi,
    Km,
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
            DistanceUnit::Mi => format!("{:.0}", self.as_miles()),
            DistanceUnit::Km => format!("{:.1}", self.as_km()),
        }
    }
}
