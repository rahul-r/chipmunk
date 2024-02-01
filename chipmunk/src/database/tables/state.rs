use anyhow::Context;
use chrono::NaiveDateTime;
use sqlx::PgPool;
use tesla_api::vehicle_data::{ChargingState, ShiftState, VehicleData};

use crate::{
    utils::location::{Distance, Location},
    utils::time_diff,
};

use super::DBTable;

#[derive(sqlx::Type, Debug, Default, Clone, Copy, Eq, PartialEq, Hash)]
#[sqlx(type_name = "states_status", rename_all = "snake_case")]
pub enum StateStatus {
    // Tesla API sends 'asleep', 'online', 'unknown', and 'offline' as vehicle states
    // Instead of using 'online' state, add 'parked', 'driving', and 'charging' as states
    // to track the vehicle's state when it is 'online'
    #[default]
    Offline,
    Asleep,
    Unknown,
    Parked,
    Driving,
    Charging,
}

impl StateStatus {
    fn from(data: &VehicleData) -> Self {
        let Some(state) = data.state.clone() else {
            log::warn!("Value of vehicle state is None");
            return Self::Unknown;
        };

        match state.as_str() {
            // These are the expected responses from tesla
            "online" => {
                // First check if the car is charging
                if let Some(state) = data
                    .charge_state
                    .as_ref()
                    .and_then(|cs| cs.charging_state.as_ref())
                {
                    match state {
                        ChargingState::Charging
                        | ChargingState::Starting
                        | ChargingState::NoPower => return Self::Charging,
                        ChargingState::Disconnected
                        | ChargingState::Stopped
                        | ChargingState::Complete
                        | ChargingState::Unknown(_) => (),
                    };
                };

                // If the car is not charging, check if driving
                let Some(ref drive_state) = data.drive_state else {
                    log::warn!("Value of drive_state is None");
                    return Self::Unknown;
                };
                match drive_state.shift_state {
                    Some(ref shift) if *shift != ShiftState::P => Self::Driving,
                    _ => Self::Parked, // If shift_state is P or None, assume the car is parked
                }
            }
            "offline" => Self::Offline,
            "asleep" => Self::Asleep,
            "unknown" => Self::Unknown,
            // If a new state is added to the tesla api, log a warning and return Unknown
            _ => {
                log::warn!("Unknown vehicle state `{state}. Consider updating `StateStatus` struct to handle this state");
                Self::Unknown
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub id: i32,
    pub state: StateStatus, // TODO: Make this optional
    pub start_date: NaiveDateTime,
    pub end_date: Option<NaiveDateTime>,
    pub car_id: i16,
}

impl Default for State {
    fn default() -> Self {
        Self {
            id: i32::default(),
            state: StateStatus::default(),
            start_date: chrono::Utc::now().naive_utc(),
            end_date: Option::default(),
            car_id: i16::default(),
        }
    }
}

impl State {
    pub fn from(data: &VehicleData, car_id: i16) -> anyhow::Result<Self> {
        Ok(State {
            id: 0,
            state: StateStatus::from(data),
            start_date: data.timestamp_utc().context("timestamp is None")?,
            end_date: None,
            car_id,
        })
    }

    /// Check if the state has changed from the previous state
    /// Returns (None, None) if the state has not changed
    /// Returns (Some(previous_state), Some(current_state)) if the state has changed


    pub fn transition(
        &self,
        previous_state: &Option<State>,
    ) -> (Option<StateStatus>, Option<StateStatus>) {
        // If there is no previous state, return start of the current state
        let Some(previous_state) = previous_state else {
            return (None, Some(self.state));
        };

        if self.state == previous_state.state {
            // If the state has not changed, return no state changes
            (None, None)
        } else {
            // If the state has changed, return end of the previous state and start of the new state
            (Some(previous_state.state), Some(self.state))
        }
    }
}

impl DBTable for State {
    fn table_name() -> &'static str {
        "states"
    }

    async fn db_get_last(pool: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"
                SELECT
                    id,
                    state AS "state!: StateStatus",
                    start_date,
                    end_date,
                    car_id
                FROM states
                ORDER BY start_date DESC LIMIT 1
            "#
        )
        .fetch_one(pool)
        .await
    }

    async fn db_get_all(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
                SELECT
                    id,
                    state AS "state!: StateStatus",
                    start_date,
                    end_date,
                    car_id
                FROM states
                ORDER BY id ASC
            "#
        )
        .fetch_all(pool)
        .await
    }

    async fn db_insert(&self, pool: &PgPool) -> sqlx::Result<i64> {
        let id = sqlx::query!(
            r#"
        INSERT INTO states
        (
            state,
            start_date,
            end_date,
            car_id
        )
        VALUES ($1, $2, $3, $4)
        RETURNING id"#,
            self.state as StateStatus,
            self.start_date,
            self.end_date,
            self.car_id,
        )
        .fetch_one(pool)
        .await?
        .id;

        Ok(id as i64)
    }

    async fn db_update(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"UPDATE states SET end_date = $1 WHERE id = $2"#,
            self.end_date,
            self.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

fn state_changed(prev_data: Option<&VehicleData>, curr_data: &VehicleData) -> bool {
    let Some(prev_data) = prev_data else { return true; };
    let Some(prev_state) = prev_data.state.as_ref() else { return false; };
    let Some(curr_state) = curr_data.state.as_ref() else { return false; };
    curr_state != prev_state
}

// We may not receive vehicle data while the car is asleep. assume sleep state if the previous
// datapoint was more than 15 minutes ago and the vehicle's position remains unchanged
fn was_asleep(prev_data: &Option<VehicleData>, curr_data: &VehicleData) -> bool {
    let Some(prev_data) = prev_data else { return false; };

    let Some(diff) = time_diff(prev_data.timestamp_utc(), curr_data.timestamp_utc()) else {
        log::error!("Error getting time difference to check vehicle state change");
        return false;
    };

    // Assume not sleep state if the last data was received less than 15 minutes ago
    if diff.num_minutes() < 15 {
        return false;
    }

    // Check if the car is at the same location as the last data point. Same location means the car
    // has not moved since reciving the last data point. It's safe to assume that the car was
    // asleep since reciveing last data point

    let Some((lat, lon)) = prev_data.location() else {
        log::warn!("Previous location is None");
        return false;
    };
    let prev_loc = Location::new(lat, lon);

    let Some((lat, lon)) = curr_data.location() else {
        log::warn!("Current location is None");
        return false;
    };
    let curr_loc = Location::new(lat, lon);

    println!(
        "{:?}, {:?}, {:?} to {:?}",
        diff.num_minutes(),
        curr_loc,
        prev_data.timestamp_utc().unwrap(),
        curr_data.timestamp_utc().unwrap()
    );

    // Check if the previous and current GPS coordinates are within 1 meter to account for any
    // minor errors in GPS coordinates
    curr_loc.within_radius(&prev_loc, Distance::from_m(1))
}

pub async fn handle_state_change(
    pool: &PgPool,
    prev_data: &Option<VehicleData>,
    curr_data: &VehicleData,
    state: &State,
) -> anyhow::Result<State> {
    let mut state = state.clone();

    if was_asleep(prev_data, curr_data) {
        if state.id != 0 {
            // Finalize the previous state data entry
            state.end_date = prev_data.as_ref().and_then(|data| data.timestamp_utc());
            if let Err(e) = state.db_update(pool).await {
                log::error!("Error updating state table: {e}");
            }
        }

        // Start a new state to record the sleep state
        state = State::from(
            prev_data.as_ref().unwrap_or(&VehicleData::default()),
            state.car_id,
        )?;
        state.state = StateStatus::Asleep;
        state.end_date = curr_data.timestamp_utc();
        state.id = state.db_insert(pool).await? as i32;

        // Since we have woken up, start a new state
        state = State::from(curr_data, state.car_id)?;
        state.id = state.db_insert(pool).await? as i32;

        return Ok(state);
    }

    if state_changed(prev_data.as_ref(), curr_data) {
        if state.id != 0 {
            // Insert end_date for current state and update the database table
            state.end_date = curr_data.timestamp_utc();
            if let Err(e) = state.db_update(pool).await {
                log::error!("Error updating state table: {e}");
            }
        }

        // Start a new state instance and insert it into the database
        state = State::from(curr_data, state.car_id)?;
        state.id = state.db_insert(pool).await? as i32;
    }

    Ok(state)
}
