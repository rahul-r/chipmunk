use anyhow::Context;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use tesla_api::{
    utils::{miles_to_km, timestamp_to_naivedatetime},
    vehicle_data::{ChargingState, VehicleData},
};

use crate::{
    database::tables::{address::Address, DBTable},
    utils::time_diff_minutes_i64,
};

use super::position::Position;

#[derive(Debug, Default, Clone)]
pub struct ChargingProcess {
    pub id: i32,
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
    pub charge_energy_added: Option<f32>,
    pub start_ideal_range_km: Option<f32>,
    pub end_ideal_range_km: Option<f32>,
    pub start_battery_level: Option<i16>,
    pub end_battery_level: Option<i16>,
    pub duration_min: Option<i16>,
    pub outside_temp_avg: Option<f32>,
    pub car_id: i16,
    pub position_id: i32,
    pub address_id: Option<i32>,
    pub start_rated_range_km: Option<f32>,
    pub end_rated_range_km: Option<f32>,
    pub geofence_id: Option<i32>,
    pub charge_energy_used: Option<f32>,
    pub cost: Option<f32>,
    pub charging_status: ChargeStat, // This is used to track the current status of charging
}

impl ChargingProcess {
    pub fn from_charges(
        // Merge this with the start() function
        charge_start: Option<&Charges>,
        charge_end: &Charges,
        car_id: i16,
        position_id: i32,
        address_id: Option<i32>,
        geofence_id: Option<i32>,
    ) -> anyhow::Result<Self> {
        let Some(charge_start) = charge_start else {
            anyhow::bail!("Charge start is None")
        };

        let charging_process = ChargingProcess {
            id: 0,
            start_date: charge_start.date.unwrap_or_default(),
            end_date: charge_end.date,
            charge_energy_added: charge_end.charge_energy_added,
            start_ideal_range_km: charge_start.ideal_battery_range_km,
            end_ideal_range_km: charge_end.ideal_battery_range_km,
            start_battery_level: charge_start.battery_level,
            end_battery_level: charge_end.battery_level,
            duration_min: time_diff_minutes_i64(charge_start.date, charge_end.date)
                .map(|x| x as i16),
            outside_temp_avg: charge_end
                .outside_temp
                .zip(charge_start.outside_temp)
                .map(|(a, b)| (a + b) / 2.0),
            start_rated_range_km: charge_start.rated_battery_range_km,
            end_rated_range_km: charge_end.rated_battery_range_km,
            geofence_id,
            charge_energy_used: None,
            cost: calculate_cost(charge_start),
            car_id,
            position_id,
            address_id,
            charging_status: ChargeStat::Done,
        };

        Ok(charging_process)
    }

    pub fn start(
        charge_start: &Charges,
        car_id: i16,
        position_id: i32,
        address_id: Option<i32>,
        geofence_id: Option<i32>,
    ) -> Self {
        ChargingProcess {
            start_date: charge_start.date.unwrap_or_default(),
            charge_energy_added: charge_start.charge_energy_added,
            start_ideal_range_km: charge_start.ideal_battery_range_km,
            start_battery_level: charge_start.battery_level,
            duration_min: Some(0),
            outside_temp_avg: charge_start.outside_temp,
            start_rated_range_km: charge_start.rated_battery_range_km,
            geofence_id,
            cost: calculate_cost(charge_start),
            car_id,
            position_id,
            address_id,
            charging_status: ChargeStat::Start,
            ..Self::default()
        }
    }

    pub fn update(&self, charges: &Charges) -> Self {
        Self {
            charge_energy_added: charges.charge_energy_added,
            duration_min: time_diff_minutes_i64(Some(self.start_date), charges.date)
                .map(|x| x as i16),
            outside_temp_avg: self
                .outside_temp_avg
                .zip(charges.outside_temp)
                .map(|(a, b)| (a + b) / 2.0),
            cost: calculate_cost(charges),
            end_date: charges.date,
            end_ideal_range_km: charges.ideal_battery_range_km,
            end_battery_level: charges.battery_level,
            end_rated_range_km: charges.rated_battery_range_km,
            charging_status: ChargeStat::Charging,
            ..self.clone()
        }
    }

    pub fn reset(&self) -> Self {
        Self {
            id: self.id,
            charging_status: ChargeStat::Done,
            ..Self::default()
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Charges {
    pub id: i32,
    pub date: Option<NaiveDateTime>,
    pub battery_heater_on: Option<bool>,
    pub battery_level: Option<i16>,
    pub charge_energy_added: Option<f32>,
    pub charger_actual_current: Option<i16>,
    pub charger_phases: Option<i16>,
    pub charger_pilot_current: Option<i16>,
    pub charger_power: Option<i16>,
    pub charger_voltage: Option<i16>,
    pub fast_charger_present: Option<bool>,
    pub conn_charge_cable: Option<String>,
    pub fast_charger_brand: Option<String>,
    pub fast_charger_type: Option<String>,
    pub ideal_battery_range_km: Option<f32>,
    pub not_enough_power_to_heat: Option<bool>,
    pub outside_temp: Option<f32>,
    pub charging_process_id: i32,
    pub battery_heater: Option<bool>,
    pub battery_heater_no_power: Option<bool>,
    pub rated_battery_range_km: Option<f32>,
    pub usable_battery_level: Option<i16>,
}

impl Charges {
    pub fn from(data: &VehicleData, charging_process_id: i32) -> anyhow::Result<Self> {
        let charge_state = data.charge_state.clone().context("charge_state is None")?;
        let climate_state = data
            .climate_state
            .clone()
            .context("climate_state is None")?;
        Ok(Self {
            id: 0,
            date: timestamp_to_naivedatetime(charge_state.timestamp),
            battery_heater_on: charge_state.battery_heater_on,
            battery_level: charge_state.battery_level,
            charge_energy_added: charge_state.charge_energy_added,
            charger_actual_current: charge_state.charger_actual_current,
            charger_phases: charge_state.charger_phases,
            charger_pilot_current: charge_state.charger_pilot_current,
            charger_power: charge_state.charger_power,
            charger_voltage: charge_state.charger_voltage,
            fast_charger_present: charge_state.fast_charger_present,
            conn_charge_cable: charge_state.conn_charge_cable,
            fast_charger_brand: charge_state.fast_charger_brand,
            fast_charger_type: charge_state.fast_charger_type,
            ideal_battery_range_km: miles_to_km(&charge_state.ideal_battery_range),
            not_enough_power_to_heat: charge_state.not_enough_power_to_heat,
            outside_temp: climate_state.outside_temp,
            charging_process_id,
            battery_heater: climate_state.battery_heater,
            battery_heater_no_power: climate_state.battery_heater_no_power,
            rated_battery_range_km: miles_to_km(&charge_state.battery_range),
            usable_battery_level: charge_state.usable_battery_level,
        })
    }
}

#[derive(Debug, PartialEq, Clone, Default, sqlx::Type)]
#[sqlx(type_name = "charge_stat", rename_all = "snake_case")]
pub enum ChargeStat {
    Start,
    Stop,
    Charging,
    Done,
    #[default]
    Unknown,
}

pub async fn db_calculate_energy_used(pool: &sqlx::PgPool, charging_process_id: i32) -> Option<f32> {
    let charges = match Charges::for_charging_process(pool, charging_process_id).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Error getting charges for charging_process: {e}");
            return None;
        }
    };
    calculate_energy_used(&charges)
}

pub fn calculate_energy_used(charges: &Vec<Charges>) -> Option<f32> {
    let phases = determine_phases(&charges);
    let mut total_energy_used = 0.0;
    let mut previous_date: Option<NaiveDateTime> = None;

    for charge in charges {
        let energy_used = match charge.charger_phases {
            Some(_) => {
                (charge.charger_actual_current.unwrap_or(0) * charge.charger_voltage.unwrap_or(0))
                    as f32
                    * phases.unwrap_or(0f32)
                    / 1000.0
            }
            None => charge.charger_power.unwrap_or(0) as f32,
        };
        println!("energy_used = {:?}", charge.charger_power);

        let time_diff =
            crate::utils::time_diff(charge.date, previous_date).unwrap_or(chrono::Duration::zero());
        total_energy_used += energy_used * (time_diff.num_seconds() as f32) / 3600.0;
        previous_date = charge.date;
    }

    Some(total_energy_used)
}

fn determine_phases(charges: &Vec<Charges>) -> Option<f32> {
    let mut total_power: f32 = 0.0;
    let mut total_phases: i32 = 0;
    let mut total_voltage: i32 = 0;
    let mut count: i32 = 0;

    for charge in charges {
        if let (Some(current), Some(voltage), Some(power)) = (
            charge.charger_actual_current,
            charge.charger_voltage,
            charge.charger_power,
        ) {
            if current != 0 {
                total_power += power as f32 * 1000.0 / (current * voltage) as f32;
            }
        }

        total_phases += charge.charger_phases.unwrap_or(0) as i32;
        total_voltage += charge.charger_voltage.unwrap_or(0) as i32;
        count += 1;
    }

    if count == 0 {
        return None;
    }

    let avg_power = total_power / count as f32;
    let avg_phases = total_phases as f32 / count as f32;
    let avg_voltage = total_voltage as f32 / count as f32;

    if avg_power > 0.0 && count > 15 {
        if avg_phases == avg_power.round() {
            Some(avg_phases)
        } else if avg_phases == 3.0 && (avg_power / f32::sqrt(avg_phases) - 1.0).abs() <= 0.1 {
            log::info!(
                "Voltage correction: {}V -> {}V",
                avg_voltage.round(),
                (avg_voltage / f32::sqrt(avg_phases)).round()
            );
            Some(f32::sqrt(avg_phases))
        } else if (avg_power.round() - avg_power).abs() <= 0.3 {
            log::info!("Phase correction: {} -> {}", avg_phases, avg_power.round());
            Some(avg_power.round())
        } else {
            None
        }
    } else {
        None
    }
}

// TODO: Add this
fn calculate_cost(_charges: &Charges) -> Option<f32> {
    None
}

/**
* Return true if battery level is more than previous data point and currently not charging
* This means the vehicle was charged while offline and we don't have any charging statistics
*/
pub fn charged_offline(
    charging_process: &ChargingProcess,
    previous: Option<&Charges>,
    current: &Charges,
) -> bool {
    match charging_process.charging_status {
        ChargeStat::Done => (),
        _ => return false, // A charging session is already in progress
    };

    let Some(previous) = previous else {
        return false;
    };

    let (Some(previous_battery_level), Some(current_battery_level)) = (previous.battery_level, current.battery_level) else {
        return false;
    };

    // Return true only if the battery level us up by at least 2% (>1). This is to prevent falsely
    // reporting battery gain by regen breaking
    current_battery_level - previous_battery_level > 1
}

pub async fn handle_charging(
    pool: &sqlx::PgPool,
    curr_vehicle_data: &VehicleData,
    previous_charge: Option<Charges>,
    current_charge: Charges,
    charging_process: ChargingProcess,
    current_position: &Position,
    car_id: i16,
) -> anyhow::Result<ChargingProcess> {
    // TODO: Run this only when the vehicle state is changing from offline to online
    if charged_offline(&charging_process, previous_charge.as_ref(), &current_charge) {
        let current_position = &Position {
            drive_id: None,
            ..current_position.clone()
        };
        let position_id = current_position.db_insert(pool).await? as i32;
        let address_id = Address::from_opt(current_position.latitude, current_position.longitude)
            .await?
            .db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}"))
            .map(|id| id as i32)
            .ok();
        let geofence_id = None; // TODO: add this
        let charging_process = ChargingProcess::from_charges(
            previous_charge.as_ref(),
            &current_charge,
            car_id,
            position_id,
            address_id,
            geofence_id,
        )?;

        log::info!("Vehicle charged offline. Inserting new charging_process `start date: {}` into database", charging_process.start_date);
        charging_process.db_insert(pool).await?;
        return Ok(ChargingProcess::default());
    }

    let start = async move |start_position: Position,
                            start_charge: Charges|
                -> anyhow::Result<ChargingProcess> {
        let start_address_id = Address::from_opt(start_position.latitude, start_position.longitude)
            .await?
            .db_insert(pool)
            .await
            .map_err(|e| log::error!("{e}"))
            .map(|id| id as i32)
            .ok();
        let start_position_id = match start_position.db_insert(pool).await {
            Ok(id) => Some(id as i32),
            Err(e) => {
                log::error!("Error adding position to database: {e}");
                None
            }
        };
        let geofence_id = None; // TODO: Add this

        let mut cp = ChargingProcess::start(
            &start_charge,
            car_id,
            start_position_id.unwrap_or(0), // TODO: position id 0 is invalid. Add a row in
            // position table for invalid position and use that position id instead of '0' here
            start_address_id,
            geofence_id,
        );

        match cp.db_insert(pool).await {
            Ok(id) => cp.id = id as i32,
            Err(e) => log::error!("Error inserting charging_process to database: {e}"),
        }

        Ok(cp)
    };

    fn charging_status(
        charging_process: &ChargingProcess,
        curr_vehicle_data: &VehicleData,
    ) -> ChargeStat {
        let Some(ref curr_charge_state) = curr_vehicle_data.charge_state else {
            return charging_process.charging_status.clone(); // Return the current state of charging
        };

        let Some(ref charging_state) = curr_charge_state.charging_state else {
            return charging_process.charging_status.clone();
        };

        let previous_charging_status = charging_process.charging_status.clone();

        match charging_state {
            ChargingState::Stopped | ChargingState::Complete | ChargingState::Disconnected => {
                match previous_charging_status {
                    ChargeStat::Stop | ChargeStat::Done => ChargeStat::Done, // If the charging process is already stopped, mark it as done
                    _ => ChargeStat::Stop, // If the charging process is not already stopped, tell logger the process to stop
                }
            }
            ChargingState::Starting | ChargingState::Charging => match previous_charging_status {
                ChargeStat::Start | ChargeStat::Charging => ChargeStat::Charging, // Charging process is already in progress, continue it
                ChargeStat::Stop | ChargeStat::Done | ChargeStat::Unknown => ChargeStat::Start, // If the charging process is not already started, tell logger the process to start logging
            },
            ChargingState::NoPower | ChargingState::Unknown(_) => previous_charging_status, // Unknown charging state, return previous state
        }
    }

    let status = charging_status(&charging_process, curr_vehicle_data);

    let mut cp = match status {
        ChargeStat::Start => start(current_position.clone(), current_charge.clone()).await?,
        ChargeStat::Charging => {
            let cp = if charging_process.id == 0 {
                start(current_position.clone(), current_charge.clone()).await?
            } else {
                let updated_cp = charging_process.update(&current_charge);
                if let Err(e) = updated_cp.db_update(pool).await {
                    log::error!("Error updating charging_process table: {e}");
                }
                updated_cp
            };

            let charge = &Charges {
                charging_process_id: cp.id,
                ..current_charge
            };

            if let Err(e) = charge.db_insert(pool).await {
                log::error!("Error inserting charging stats to database: {e}");
            }

            cp
        }
        ChargeStat::Stop => {
            let cp = ChargingProcess {
                charging_status: ChargeStat::Done,
                charge_energy_used: db_calculate_energy_used(pool, charging_process.id).await,
                ..charging_process.update(&current_charge)
            };

            // FIXME: This case gets called multiple times which causes database insertion error because the `.reset()` function of
            // charging process sets the position_id to 0 which is invalid.
            // As a workaround, we are checking if the position_id is 0 before continuing.
            if charging_process.position_id != 0 {
                if let Err(e) = cp.db_update(pool).await {
                    log::error!("Error updating charging_process table: {e}");
                }
            }

            cp.reset()
        }
        ChargeStat::Done | ChargeStat::Unknown => charging_process,
    };

    cp.charging_status = status;
    Ok(cp)
}
