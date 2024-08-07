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

// Struct to store distance
// Internally this struct stores distance in miles
type DistanceType = f64;
#[derive(Debug, Default, PartialEq, PartialOrd, Clone, Serialize, Deserialize)]
pub struct Distance(DistanceType);

impl Distance {
    // Create Distance from meters
    pub fn from_m<T: Into<DistanceType>>(m: T) -> Self {
        Distance(m.into() / 1_609.344)
    }

    /// Create Distance from kilometers
    pub fn from_km<T: Into<DistanceType>>(km: T) -> Self {
        Distance(km.into() / 1.609_344)
    }

    /// Create Distance from miles
    pub fn from_miles<T: Into<DistanceType>>(miles: T) -> Self {
        Distance(miles.into())
    }

    /// Get meters
    pub fn as_m(&self) -> DistanceType {
        self.0 * 1_609.344
    }

    /// Get kilometers
    pub fn as_km(&self) -> DistanceType {
        self.0 * 1.609_344
    }

    /// Get miles
    pub fn as_miles(&self) -> DistanceType {
        self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0.0
    }

    pub fn to_string(&self, unit: &DistanceUnit) -> String {
        match unit {
            DistanceUnit::Mi => format!("{}", self.as_miles().round()),
            DistanceUnit::Km => format!("{}", self.as_km().round()),
        }
    }

    pub fn round(&self) -> Distance {
        Distance(self.0.round())
    }
}
