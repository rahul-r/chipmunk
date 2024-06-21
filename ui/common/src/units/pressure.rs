use std::str::FromStr;

use serde::{Deserialize, Serialize};

// NOTE: Make sure this enum matches UnitOfTemperature enum in chipmunk/src/database/types.rs
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum PressureUnit {
    #[default]
    Psi,
    Bar,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pressure {
    pub psi: f32,
}

impl Pressure {
    // TODO: Implement pressure conversions
}
