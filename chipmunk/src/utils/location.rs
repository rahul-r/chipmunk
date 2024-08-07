use std::ops::Sub;
use ui_common::units::Distance;

const EARTH_RADIUS_KM: f64 = 6367.5;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

impl Sub for Location {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            lat: self.lat - other.lat,
            lon: self.lon - other.lon,
        }
    }
}

impl Location {
    pub fn new<T: Into<f64>>(lat: T, lon: T) -> Self {
        Self {
            lat: lat.into(),
            lon: lon.into(),
        }
    }

    pub fn to_radians(&self) -> Self {
        Self {
            lat: self.lat.to_radians(),
            lon: self.lon.to_radians(),
        }
    }

    pub fn distance_to(&self, other: &Self) -> Distance {
        haversine(self, other)
    }

    pub fn within_radius(&self, other: &Self, radius: Distance) -> bool {
        haversine(self, other) <= radius
    }
}

// See https://en.wikipedia.org/wiki/Great-circle_distance
fn haversine(loc1: &Location, loc2: &Location) -> Distance {
    let loc1 = loc1.to_radians();
    let loc2 = loc2.to_radians();

    let diff = loc1 - loc2;

    let a = (diff.lat / 2.0).sin().powi(2)
        + (diff.lon / 2.0).sin().powi(2) * loc1.lat.cos() * loc2.lat.cos();

    let c = 2.0 * a.sqrt().asin();

    Distance::from_km(c * EARTH_RADIUS_KM)
}

#[test]
fn test_haversine() {
    let loc1 = Location {
        lat: 57.3645,
        lon: -110.568,
    };
    let loc2 = Location {
        lat: 60.456,
        lon: -111.6456,
    };
    assert_eq!(
        loc1.distance_to(&loc2).round(),
        Distance::from_km(349.0).round()
    );
}
