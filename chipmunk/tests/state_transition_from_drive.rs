pub mod common;

use chipmunk::database::tables::{state::{State, StateStatus}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_charging, data_with_shift, data_with_state}, utils::ts_no_nanos, DELAYED_DATAPOINT_TIME_SEC};

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
    use StateStatus::*;
    let car_id = 1i16;
    
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

    // Drive to park
    let start_time = chrono::Utc::now().naive_utc();
    let drive_tables = chipmunk::logger::create_tables(&data_with_shift(start_time, Some(D)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(drive_tables.len(), 1);
    assert!(drive_tables[0].address.is_some());
    assert!(drive_tables[0].car.is_none());
    assert_eq!(drive_tables[0].charges.as_ref().unwrap().id, 0);
    assert!(drive_tables[0].charging_process.is_none());
    assert!(drive_tables[0].drive.is_some());
    assert!(drive_tables[0].position.is_some());
    assert!(drive_tables[0].settings.is_none());
    assert!(drive_tables[0].sw_update.is_none());
    assert!(drive_tables[0].state.is_some());
    assert_eq!(*drive_tables[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(start_time), end_date: None });
    let prev_tables = &drive_tables[0];
    let ts = start_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(P)), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(start_time), end_date:  Some(ts_no_nanos(start_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert_eq!(t[1].charges.as_ref().unwrap().id, 0);
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Drive to asleep
    let t = chipmunk::logger::create_tables(&data_with_state(ts, Asleep), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(start_time), end_date:  Some(ts_no_nanos(start_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert_eq!(t[1].charges.as_ref().unwrap().id, 0);
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Drive to offline
    let t = chipmunk::logger::create_tables(&data_with_state(ts, Offline), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(start_time), end_date:  Some(ts_no_nanos(start_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert_eq!(t[1].charges.as_ref().unwrap().id, 0);
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Drive to charging
    let t = chipmunk::logger::create_tables(&data_charging(ts), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(start_time), end_date:  Some(ts_no_nanos(start_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert_eq!(t[1].charges.as_ref().unwrap().id, 0);
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());
}