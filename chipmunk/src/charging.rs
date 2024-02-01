use chrono::NaiveDateTime;

use crate::database::tables::charges::Charges;

pub fn calculate_energy_used(charges: &Vec<Charges>) -> Option<f32> {
    let phases = determine_phases(charges);
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
pub fn calculate_cost(_charges: &Charges) -> Option<f32> {
    None
}

// async fn calculate_efficiency(pool: &PgPool) -> anyhow::Result<f32> {
//     // TODO: Get preferred_range from settings table
//     let preferred_range = Range::Rated;
//     let charging_processes = ChargingProcess::db_get(pool).await?;
//     let car_id = 0;

//     fn calculate(cp: ChargingProcess, car_id: i16, preferred_range: Range) -> anyhow::Result<f32> {
//         let start_range = match preferred_range {
//             Range::Ideal => cp.start_rated_range_km,
//             Range::Rated => cp.start_ideal_range_km,
//         };
//         let end_range = match preferred_range {
//             Range::Ideal => cp.end_rated_range_km,
//             Range::Rated => cp.end_ideal_range_km,
//         };

//         if cp.car_id == car_id
//             && cp.duration_min.context("unexpected duration")? > 10
//             && cp.end_battery_level.context("unexpected battery level")? > 95
//             && cp.charge_energy_added.context("unexpected charge energy")? > 0.0
//         {
//             if let (Some(energy), Some(end_range), Some(start_range)) =
//                 (cp.charge_energy_added, end_range, start_range)
//             {
//                 return Ok(energy / (end_range - start_range));
//             }
//         }

//         anyhow::bail!("Cannot calculate efficiency");
//     }

//     for cp in charging_processes {
//         match calculate(cp, car_id, preferred_range) {
//             Ok(efficiency) => return Ok(efficiency),
//             Err(e) => log::error!("{e}"),
//         }
//     }

//     anyhow::bail!("Cannot calculate efficiency");
// }
