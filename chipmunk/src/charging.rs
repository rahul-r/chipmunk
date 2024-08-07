use chrono::{DateTime, Duration, Utc};

use crate::database::tables::charges::Charges;

// FIXME: This function produces a different result than Teslamate. Need to figure out why.
pub fn calculate_energy_used(charges: &[Charges]) -> Option<f32> {
    let phases = determine_phases(charges);
    let mut total_energy_used = 0.0;
    let mut previous_date: Option<DateTime<Utc>> = None;

    for charge in charges {
        let energy_used = if charge.charger_phases.is_some() {
            charge
                .charger_actual_current
                .zip(charge.charger_voltage)
                .zip(phases)
                .map_or(0.0, |((current, voltage), phases)| {
                    (current * voltage) as f32 * phases / 1000.0
                })
        } else {
            charge.charger_power.unwrap_or(0) as f32
        };

        if charge.date.is_none() {
            log::warn!("No timestamp found in charge");
        }

        let time_diff = charge
            .date
            .zip(previous_date)
            .map_or(Duration::zero(), |(c, p)| c - p)
            .num_seconds();
        total_energy_used += energy_used * (time_diff as f32) / 3600.0;
        previous_date = charge.date;
    }

    Some(total_energy_used)
}

fn determine_phases(charges: &[Charges]) -> Option<f32> {
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

// TODO: Implement this
pub fn calculate_cost(_charges: &Charges) -> Option<f32> {
    None
}
