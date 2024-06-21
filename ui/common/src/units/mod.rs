pub mod distance;
pub mod pressure;
pub mod temperature;

pub use distance::{Distance, DistanceUnit};
pub use pressure::{Pressure, PressureUnit};
use serde::{Deserialize, Serialize};
pub use temperature::{Temperature, TemperatureUnit};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "measurement", content = "unit")]
pub enum Measurement {
    Distance(DistanceUnit),
    Pressure(PressureUnit),
    Temperature(TemperatureUnit),
}
