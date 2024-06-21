use ui_common::units::{DistanceUnit, PressureUnit, TemperatureUnit};

// Postgres types
#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "billing_type", rename_all = "snake_case")]
pub enum BillingType {
    #[default]
    PerKwh,
    PerMinute,
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_length", rename_all = "lowercase")]
pub enum UnitOfLength {
    Km,
    #[default]
    Mi,
}

impl UnitOfLength {
    pub fn from_ui_struct(unit: &DistanceUnit) -> Self {
        match unit {
            DistanceUnit::Mi => Self::Mi,
            DistanceUnit::Km => Self::Km,
        }
    }

    pub fn to_ui_struct(&self) -> DistanceUnit {
        match self {
            UnitOfLength::Km => DistanceUnit::Km,
            UnitOfLength::Mi => DistanceUnit::Mi,
        }
    }
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_pressure", rename_all = "lowercase")]
pub enum UnitOfPressure {
    Bar,
    #[default]
    Psi,
}

impl UnitOfPressure {
    pub fn from_ui_struct(unit: &PressureUnit) -> Self {
        match unit {
            PressureUnit::Psi => Self::Psi,
            PressureUnit::Bar => Self::Bar,
        }
    }

    pub fn to_ui_struct(&self) -> PressureUnit {
        match self {
            UnitOfPressure::Bar => PressureUnit::Bar,
            UnitOfPressure::Psi => PressureUnit::Psi,
        }
    }
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "unit_of_temperature")]
pub enum UnitOfTemperature {
    C,
    #[default]
    F,
}

impl UnitOfTemperature {
    pub fn from_ui_struct(unit: &TemperatureUnit) -> Self {
        match unit {
            TemperatureUnit::F => Self::F,
            TemperatureUnit::C => Self::C,
        }
    }

    pub fn to_ui_struct(&self) -> TemperatureUnit {
        match self {
            UnitOfTemperature::C => TemperatureUnit::C,
            UnitOfTemperature::F => TemperatureUnit::F,
        }
    }
}

#[derive(sqlx::Type, Debug, Default, Clone, Copy)]
#[sqlx(type_name = "range", rename_all = "lowercase")]
pub enum Range {
    Ideal,
    #[default]
    Rated,
}

#[derive(Debug, PartialEq, Clone, Default, sqlx::Type)]
#[sqlx(type_name = "charge_stat", rename_all = "snake_case")]
pub enum ChargeStat {
    #[default]
    Start,
    Charging,
    Done,
}
