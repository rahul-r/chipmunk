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
    pub fn from_psi(psi: f32) -> Self {
        Pressure { psi }
    }

    pub fn from_bar(bar: f32) -> Self {
        Self { psi: bar * 14.504 }
    }

    pub fn to_psi(&self) -> f32 {
        self.psi
    }

    pub fn to_bar(&self) -> f32 {
        self.psi * 0.068948
    }
}
