pub mod common;

use chipmunk::database::tables::{state::{State, StateStatus}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::data_with_shift, utils::ts_no_nanos, DELAYED_DATAPOINT_TIME_SEC};


#[tokio::test]
async fn state_change_from_parked() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;

    // Create initial parked state
    let parking_start_time = chrono::Utc::now().naive_utc();
    let t = chipmunk::logger::create_tables(&data_with_shift(parking_start_time, Some(P)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let parked_state = &t[0];

    // Test state changes from parked state to parked state
    let ts = parking_start_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC - 1);
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(P)), parked_state, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(ts)) });

    // Test state changes from shift state P to null
    let ts = parking_start_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC - 1);
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, None), parked_state, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(ts)) });

    // Test state changes from parked state to driving state
    let parking_end_time = parking_start_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC - 1);
    let driving_start_time = parking_end_time + Duration::seconds(1);
    let mut s = parked_state.clone();
    s.position.as_mut().unwrap().date = Some(ts_no_nanos(parking_end_time));
    let t = chipmunk::logger::create_tables(&data_with_shift(driving_start_time, Some(D)), &s, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    // End of Parked state
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_end_time)) });
    // Start of Driving state
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].sw_update.is_none());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: None });

    // Test state changes from parked state to reverse state
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(R)), parked_state, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    // End of Parked state
    assert!(t[0].drive.is_none());
    // NOTE: the logging process will use the previous data point to stop a state, in this case
    // previous data point is also the starting sata point of parking state. state start time and
    // state end time will be same.
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_start_time)) });
    // Start of Driving state
    assert!(t[1].drive.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // Test state changes from parked state to neutral state
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(N)), parked_state, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].drive.is_none());
    assert!(t[1].drive.is_some());
    // End of Parked state
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_start_time)) });
    // Start of Driving state
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // TODO:
    // Parked to asleep

    // Parked to offline

    // Parked to charging
}