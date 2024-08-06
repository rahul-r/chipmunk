#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::too_many_lines)]

pub mod common;

use chipmunk::{database::tables::{state::{State, StateStatus}, Tables}, DELAYED_DATAPOINT_TIME_SEC};
use chrono::Duration;
use chipmunk::task_data_processor::create_tables;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_charging, data_with_shift, data_with_state}, utils::ts_no_nanos};


#[tokio::test]
async fn state_change_from_parked() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;

    // Create initial parked state
    let parking_start_time = chrono::Utc::now();
    let t = create_tables(&data_with_shift(parking_start_time, Some(P)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let parked_tables = &t[0];

    // Test state changes from parked state to parked state
    let ts = parking_start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC).unwrap();
    let t = create_tables(&data_with_shift(ts, Some(P)), parked_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(ts)) });

    // Test state changes from shift state P to null
    let ts = parking_start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC).unwrap();
    let t = create_tables(&data_with_shift(ts, None), parked_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(ts)) });

    // Test state changes from parked state to driving state
    let parking_end_time = parking_start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC).unwrap();
    let driving_start_time = parking_end_time + Duration::try_seconds(1).unwrap();
    let parked_tables_1 = Tables {
        time: Some(ts_no_nanos(parking_end_time)),
        ..parked_tables.clone()
    };
    let t = create_tables(&data_with_shift(driving_start_time, Some(D)), &parked_tables_1, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    // End of Parked state
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_end_time)) });
    // Start of Driving state
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].sw_update.is_none());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: None });

    // Test state changes from parked state to reverse state
    let t = create_tables(&data_with_shift(ts, Some(R)), parked_tables, car_id).await.unwrap();
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
    let t = create_tables(&data_with_shift(ts, Some(N)), parked_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].drive.is_none());
    assert!(t[1].drive.is_some());
    // End of Parked state
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_start_time)) });
    // Start of Driving state
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // Parked to asleep
    let parking_end_time = parking_start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_shift(parking_end_time, Some(P)), &t[0], car_id).await.unwrap();
    let parked_state1 = &t[0];
    
    let sleep_start_time = parking_end_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_state(sleep_start_time, Asleep), parked_state1, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(sleep_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Parked to offline
    let parking_end_time = parking_start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_shift(parking_end_time, Some(P)), &t[0], car_id).await.unwrap();
    let parked_tables_2 = &t[0];
    
    let offline_start_time = parking_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_with_state(offline_start_time, Offline), parked_tables_2, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Parked to charging
    let parking_end_time = parking_start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_shift(parking_end_time, Some(P)), &t[0], car_id).await.unwrap();
    let parked_tables_3 = &t[0];
    
    let charging_start_time = parking_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_charging(charging_start_time, 25), parked_tables_3, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: Some(ts_no_nanos(parking_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());
}