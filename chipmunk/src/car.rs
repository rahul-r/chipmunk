use anyhow::Context;

use crate::database::{tables::charging_process::ChargingProcess, types::Range};


// TODO: Test if this function works as expected
pub fn calculate_efficiency(cp_list: &[ChargingProcess], car_id: i16, preferred_range: Range) -> anyhow::Result<f32> {
    fn calculate(cp: &ChargingProcess, car_id: i16, preferred_range: Range) -> anyhow::Result<f32> {
        let start_range = match preferred_range {
            Range::Ideal => cp.start_rated_range_km,
            Range::Rated => cp.start_ideal_range_km,
        };
        let end_range = match preferred_range {
            Range::Ideal => cp.end_rated_range_km,
            Range::Rated => cp.end_ideal_range_km,
        };

        if cp.car_id == car_id
            && cp.duration_min.context("unexpected duration")? > 10
            && cp.end_battery_level.context("unexpected battery level")? > 95
            && cp.charge_energy_added.context("unexpected charge energy")? > 0.0
        {
            if let (Some(energy), Some(end_range), Some(start_range)) =
                (cp.charge_energy_added, end_range, start_range)
            {
                return Ok(energy / (end_range - start_range));
            }
        }

        anyhow::bail!("Cannot calculate efficiency");
    }

    for cp in cp_list {
        match calculate(cp, car_id, preferred_range) {
            Ok(efficiency) => return Ok(efficiency),
            Err(e) => log::error!("{e}"),
        }
    }

    anyhow::bail!("Cannot calculate efficiency");
}
