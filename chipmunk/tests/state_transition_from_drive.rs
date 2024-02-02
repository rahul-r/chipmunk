pub mod common;

use chipmunk::database::tables::{state::{State, StateStatus}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::data_with_shift, utils::ts_no_nanos, DELAYED_DATAPOINT_TIME_SEC};

async fn test_drive_to_drive_states(from_shift: ShiftState, to_shift: ShiftState) {
    use StateStatus::*;
    let car_id = 1i16;
    // Create initial driving state
    let start_time = chrono::Utc::now().naive_utc();
    let driving_start_time = start_time;
    let t = chipmunk::logger::create_tables(&data_with_shift(start_time, Some(from_shift)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_state = &t[0];

    let curr_data_time = start_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC - 1);

    let t = chipmunk::logger::create_tables(&data_with_shift(curr_data_time, Some(to_shift)), prev_state, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: Some(ts_no_nanos(curr_data_time)) });
}

#[tokio::test]
async fn state_change_from_driving() {
    use ShiftState::*;
    // Test state changes from D, R, N to D, R, N (vehicle should stay in drive state when switching between these states)
    test_drive_to_drive_states(D, D).await;
    test_drive_to_drive_states(D, R).await;
    test_drive_to_drive_states(D, N).await;
    test_drive_to_drive_states(R, D).await;
    test_drive_to_drive_states(R, R).await;
    test_drive_to_drive_states(R, N).await;
    test_drive_to_drive_states(N, D).await;
    test_drive_to_drive_states(N, R).await;
    test_drive_to_drive_states(N, N).await;

    // TODO:
    // Drive to park

    // Drive to asleep

    // Drive to offline

    // Drive to charging
}