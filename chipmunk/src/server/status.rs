use ui_common::{
    units::{Distance, DistanceUnit, PressureUnit, Temperature, TemperatureUnit},
    Charging, Driving, Location, Logging, Offline, Parked, Sleeping, State, Status, Vehicle,
};

use crate::{
    config::Config,
    database::{
        tables::Tables,
        types::{UnitOfLength, UnitOfPressure, UnitOfTemperature},
    },
};

fn driving(tables: &Tables, state: &State, curr_status: Option<&Driving>) -> Option<Driving> {
    if *state != State::Driving {
        return None;
    }

    let status = if let Some(curr_status) = curr_status {
        let current_charge = tables.charges.as_ref().and_then(|c| c.battery_level);

        Driving {
            start_time: curr_status.start_time,
            duration_sec: 0,
            starting_battery_level: curr_status.starting_battery_level,
            current_battery_level: current_charge,
            miles_driven: 0,
            charge_used: curr_status
                .starting_battery_level
                .zip(current_charge)
                .map_or(curr_status.charge_used, |(starting, current)| {
                    starting - current
                }),
            destination: curr_status.destination.clone(),
            time_remaining_sec: 0,
            battery_level_at_destination: curr_status.battery_level_at_destination,
        }
    } else {
        Driving {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
            starting_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            current_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            miles_driven: 0,
            charge_used: 0,
            destination: None,
            time_remaining_sec: 0,
            battery_level_at_destination: 0.,
        }
    };

    Some(status)
}

fn charging(tables: &Tables, state: &State, curr_status: Option<&Charging>) -> Option<Charging> {
    if *state != State::Charging {
        return None;
    }

    let status = if let Some(curr_status) = curr_status {
        Charging {
            start_time: curr_status.start_time,
            duration_sec: (chrono::offset::Utc::now() - curr_status.start_time).num_seconds(),
            starting_battery_level: curr_status.starting_battery_level,
            current_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            charge_added: tables
                .charging_process
                .as_ref()
                .and_then(|c| c.charge_energy_added)
                .unwrap_or(curr_status.charge_added),
            cost: 0, // TODO: calculate cost
            time_remaining_sec: curr_status.time_remaining_sec,
        }
    } else {
        Charging {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
            starting_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            current_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            charge_added: 0f32,
            cost: 0,
            time_remaining_sec: 0,
        }
    };

    Some(status)
}

fn parked(tables: &Tables, state: &State, curr_status: Option<&Parked>) -> Option<Parked> {
    if *state != State::Parked {
        return None;
    }

    let status = if let Some(curr_status) = curr_status {
        let current_charge = tables.charges.as_ref().and_then(|c| c.battery_level);

        Parked {
            start_time: curr_status.start_time,
            duration_sec: 0,
            starting_battery_level: curr_status.starting_battery_level,
            current_battery_level: current_charge,
            charge_used: curr_status
                .starting_battery_level
                .zip(current_charge)
                .map_or(curr_status.charge_used, |(starting, current)| {
                    starting - current
                }),
        }
    } else {
        Parked {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
            starting_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            current_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            charge_used: 0,
        }
    };

    Some(status)
}

fn offline(state: &State, curr_status: Option<&Offline>) -> Option<Offline> {
    if *state != State::Offline {
        return None;
    }

    let status = if let Some(curr_status) = curr_status {
        Offline {
            start_time: curr_status.start_time,
            duration_sec: (chrono::offset::Utc::now() - curr_status.start_time).num_seconds(),
        }
    } else {
        Offline {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
        }
    };

    Some(status)
}

fn sleeping(state: &State, curr_status: Option<&Sleeping>) -> Option<Sleeping> {
    if *state != State::Sleeping {
        return None;
    }

    let status = if let Some(curr_status) = curr_status {
        // Already in sleep state, update the duration
        Sleeping {
            start_time: curr_status.start_time,
            duration_sec: (chrono::offset::Utc::now() - curr_status.start_time).num_seconds(),
        }
    } else {
        // Start sleep state
        Sleeping {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
        }
    };

    Some(status)
}

fn vehicle(tables: &Tables, curr_status: &Vehicle) -> Vehicle {
    // TODO: Also update the location when the state changes
    let location_name = match curr_status.location.name {
        Some(ref l) => Some(l.clone()),
        None => tables.address.as_ref().and_then(|a| a.display_name.clone()),
    };

    let location = tables.raw_data.as_ref().and_then(|d| d.location());

    let vehicle_state = tables
        .raw_data
        .as_ref()
        .and_then(|d| d.vehicle_state.as_ref());

    let climate_state = tables
        .raw_data
        .as_ref()
        .and_then(|d| d.climate_state.as_ref());

    Vehicle {
        name: vehicle_state
            .and_then(|v| v.vehicle_name.clone())
            .unwrap_or_default(),
        odometer: vehicle_state
            .and_then(|v| v.odometer)
            .map(|o| Distance::from_miles(o as f64))
            .unwrap_or_default(),
        is_user_nearby: false,
        is_locked: vehicle_state.and_then(|v| v.locked),
        location: Location {
            name: location_name,
            coords: location,
        },
        battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
        interior_temperature: climate_state
            .and_then(|c| c.inside_temp)
            .map(Temperature::from_celsius),
        exterior_temperature: climate_state
            .and_then(|c| c.outside_temp)
            .map(Temperature::from_celsius),
        range: tables
            .charges
            .as_ref()
            .and_then(|c| c.rated_battery_range_km)
            .map(|r| Distance::from_km(r as f64)),
        climate_control_state: Some(ui_common::ClimateState::default()), // TODO: replace default
                                                                         // with actual climate state
    }
}

fn logging(curr_status: Option<&Logging>, config: &Config) -> Logging {
    Logging {
        enabled: config
            .logging_enabled
            .lock()
            .map(|l| l.get())
            .map_err(|e| log::error!("{e}"))
            .unwrap_or(true),
        current_num_points: curr_status.map_or(0, |s| s.current_num_points + 1),
        total_num_points: curr_status.map_or(0, |s| s.total_num_points + 1),
        unit_of_length: DistanceUnit::default(),
        unit_of_temperature: TemperatureUnit::default(),
        unit_of_pressure: PressureUnit::default(),
    }
}

#[derive(Clone)]
pub struct LoggingStatus {
    status: Status,
}

impl LoggingStatus {
    pub fn new(config: &Config, tables: &Tables) -> Self {
        let state = State::default();
        let curr_status = Status::default();

        Self {
            status: Status {
                timestamp: chrono::offset::Utc::now(),
                app_start_time: chrono::offset::Utc::now(),
                state: state.clone(),
                logging: logging(None, config),
                vehicle: vehicle(tables, &curr_status.vehicle),
                driving: driving(tables, &state, curr_status.driving.as_ref()),
                charging: charging(tables, &state, curr_status.charging.as_ref()),
                parked: parked(tables, &state, curr_status.parked.as_ref()),
                offline: offline(&state, curr_status.offline.as_ref()),
                sleeping: sleeping(&state, curr_status.sleeping.as_ref()),
            },
        }
    }

    pub fn update(&mut self, tables: &Tables, config: &Config) {
        let state = if let Some(s) = tables.state.as_ref().map(|s| s.state) {
            use crate::database::tables::state::StateStatus as ss;
            match s {
                ss::Offline => State::Offline,
                ss::Asleep => State::Sleeping,
                ss::Unknown => State::Unknown,
                ss::Parked => State::Parked,
                ss::Driving => State::Driving,
                ss::Charging => State::Charging,
            }
        } else {
            State::Unknown
        };

        self.status = Status {
            timestamp: chrono::offset::Utc::now(),
            app_start_time: chrono::offset::Utc::now(),
            state: state.clone(),
            logging: logging(Some(&self.status.logging), config),
            vehicle: vehicle(tables, &self.status.vehicle),
            driving: driving(tables, &state, self.status.driving.as_ref()),
            charging: charging(tables, &state, self.status.charging.as_ref()),
            parked: parked(tables, &state, self.status.parked.as_ref()),
            offline: offline(&state, self.status.offline.as_ref()),
            sleeping: sleeping(&state, self.status.sleeping.as_ref()),
        };
    }

    pub fn to_value(&self) -> anyhow::Result<serde_json::Value> {
        self.status.to_value()
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        self.status.to_string()
    }

    pub fn set_unit_of_length(&mut self, value: UnitOfLength) {
        self.status.logging.unit_of_length = value.to_ui_struct();
    }

    pub fn set_unit_of_temperature(&mut self, value: UnitOfTemperature) {
        self.status.logging.unit_of_temperature = value.to_ui_struct();
    }

    pub fn set_unit_of_pressure(&mut self, value: UnitOfPressure) {
        self.status.logging.unit_of_pressure = value.to_ui_struct();
    }

    pub fn set_logging_status(&mut self, status: bool) {
        self.status.logging.enabled = status;
    }
}
