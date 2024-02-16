pub mod common;

use chipmunk::database::tables::{state::{StateStatus, State}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_charging, data_with_shift}, utils::ts_no_nanos, DELAYED_DATAPOINT_TIME_SEC};

#[tokio::test]
async fn startup_with_state() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;

    let ts = chrono::Utc::now().naive_utc();
    let blank_tables = Tables::default();

    // Test startup with shift state null (expect parked state)
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, None), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(ts), end_date: None });

    // Test startup with shift state P (expect parked state)
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(P)), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(ts), end_date: None });

    // Test startup with shift state D (expect driving state)
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(D)), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // Test startup with shift state R (expect driving state)
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(R)), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // Test startup with shift state N (expect driving state)
    let t = chipmunk::logger::create_tables(&data_with_shift(ts, Some(N)), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // Test starting when a charging session is active
    let t = chipmunk::logger::create_tables(&data_charging(ts, 95), &blank_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(ts), end_date: None });
}

// Test behavior of the logger when the data points are more thabn 10 minutes apart.
// In most cases, the logger end the previous state and start a new state.
// PREVIOUS_STATE,  CURRENT_STATE,                          EXPECTED_BEHAVIOR
// Driving,         NotDriving,                             Previous drive stopped and no new drive is created
// Driving,         Driving,                                Previous drive stopped and a new drive is created
// Driving,         Driving & battery level is higher,      Previous drive stopped and a new drive is created, new charging process is created
// NotDriving,      Driving & battery level is higher,      New drive and new charging process is created
// NotDriving,      NotDriving & battery level is higher,   No change to Drive table, a new charging process is created
#[tokio::test]
async fn state_transitions_with_time_gap() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;

    // Create driving datapoint
    let first_ts = chrono::Utc::now().naive_utc();
    let first_data_point = chipmunk::logger::create_tables(&data_with_shift(first_ts, Some(D)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(first_data_point.len(), 1);
    assert_eq!(*first_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: None });

    // Create another driving data point with time gap equal to delayed datapoint threshold
    let second_ts = first_ts + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states when time between driving data points is equal to the threshold
    assert_eq!(second_data_point.len(), 1);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(second_ts)) });

    // Create another driving data point after the delayed datapoint threshold
    let second_ts = first_ts + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states
    assert_eq!(second_data_point.len(), 2);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(first_ts)) });
    assert_eq!(*second_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(second_ts), end_date: None });

    // Create another driving data point after the delayed datapoint threshold
    let second_ts = first_ts  + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states when time between the driving data points are more than the threshold
    assert_eq!(second_data_point.len(), 2);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(first_ts)) });
    assert_eq!(*second_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(second_ts), end_date: None });

    // Test state changes when a data point is received after the threshold while in driving state and the car hasn't moved
    let second_ts = first_ts + Duration::minutes(1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    let third_ts = second_ts + Duration::minutes(1);
    let third_data_point = chipmunk::logger::create_tables(&data_with_shift(third_ts, Some(D)), &second_data_point[0], car_id).await.unwrap();
    let fourth_ts = third_ts + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let fourth_data_point = chipmunk::logger::create_tables(&data_with_shift(fourth_ts, Some(D)), &third_data_point[0], car_id).await.unwrap();
    // Verify the states
    assert_eq!(second_data_point.len(), 1);
    assert_eq!(third_data_point.len(), 1);
    assert_eq!(fourth_data_point.len(), 2);
    // Check for end of first drive
    assert_eq!(*fourth_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(third_ts)) });
    // Check for start of new drive
    assert_eq!(*fourth_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(fourth_ts), end_date: None });

    // See charging_process.rs for more tests on missing charging process detection when data is delayed
}