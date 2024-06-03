use ui_common::{Charging, Driving, Logging, Offline, Parked, Sleeping, State, Status, Vehicle};

use crate::{config::Config, database::tables::Tables};

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
                .map(|(starting, current)| starting - current)
                .unwrap_or(curr_status.charge_used),
            destination: None, // TODO: Insert destination
            time_remaining_sec: 0,
            battery_level_at_destination: 0.,
        }
    } else {
        Driving {
            start_time: chrono::offset::Utc::now(),
            duration_sec: 0,
            starting_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            current_battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
            miles_driven: 0,
            charge_used: 0,
            destination: None, // TODO: Insert destination
            time_remaining_sec: 0,
            battery_level_at_destination: 0., // TODO: Insert charge at destination
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
            cost: 0,               // TODO: calculate cost
            time_remaining_sec: 0, // TODO: calculate time remaining
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
                .map(|(starting, current)| starting - current)
                .unwrap_or(curr_status.charge_used),
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
    let location = match curr_status.location {
        Some(ref l) => Some(l.clone()),
        None => tables.address.as_ref().and_then(|a| a.display_name.clone()),
    };

    Vehicle {
        odometer: tables
            .raw_data
            .as_ref()
            .and_then(|d| d.vehicle_state.as_ref())
            .and_then(|v| v.odometer)
            .unwrap_or_default(),
        is_user_nearby: false,
        location,
        battery_level: tables.charges.as_ref().and_then(|c| c.battery_level),
        interior_temperature: tables
            .raw_data
            .as_ref()
            .and_then(|d| d.climate_state.as_ref())
            .and_then(|c| c.inside_temp),
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
        current_num_points: curr_status.map(|s| s.current_num_points + 1).unwrap_or(0),
        total_num_points: curr_status.map(|s| s.total_num_points + 1).unwrap_or(0),
    }
}

#[derive(Clone)]
pub struct LoggingStatus {
    status: Status,
}

impl LoggingStatus {
    pub fn new(config: &Config) -> Self {
        let state = State::default();
        let curr_status = Status::default();
        let tables = Tables::default();

        Self {
            status: Status {
                timestamp: chrono::offset::Utc::now(),
                app_start_time: chrono::offset::Utc::now(),
                state: state.clone(),
                logging: logging(None, config),
                vehicle: vehicle(&tables, &curr_status.vehicle),
                driving: driving(&tables, &state, curr_status.driving.as_ref()),
                charging: charging(&tables, &state, curr_status.charging.as_ref()),
                parked: parked(&tables, &state, curr_status.parked.as_ref()),
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

    pub fn set_logging_status(&mut self, status: bool) {
        self.status.logging.enabled = status;
    }
}
