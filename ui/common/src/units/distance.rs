use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfLength enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum DistanceUnit {
    #[default]
    Mi,
    Km,
}

impl DistanceUnit {
    pub fn to_str<'a>(&self) -> &'a str {
        match self {
            DistanceUnit::Mi => "mi",
            DistanceUnit::Km => "km",
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
            DistanceUnit::Mi => format!("{}", self.as_miles().round()),
            DistanceUnit::Km => format!("{}", self.as_km().round()),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.miles == 0.0
    }
}
