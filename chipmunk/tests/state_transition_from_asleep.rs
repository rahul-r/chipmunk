#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::too_many_lines)]

pub mod common;

use chipmunk::{database::tables::{state::{State, StateStatus}, Tables}, DELAYED_DATAPOINT_TIME_SEC};
use chrono::Duration;
use chipmunk::task_data_processor::create_tables;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_with_state, data_with_shift, data_charging}, utils::ts_no_nanos};

#[tokio::test]
async fn test1() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;
    chipmunk::init_log();

    // Asleep to asleep
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(start_time), end_date: None });
    assert!(t[0].sw_update.is_none());
    let prev_state = &t[0];

    let ts = start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_state(ts, Asleep), prev_state, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(start_time), end_date: Some(ts_no_nanos(ts)) });
    assert!(t[0].sw_update.is_none());

    let ts = start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_with_state(ts, Asleep), prev_state, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(start_time), end_date: Some(ts_no_nanos(ts)) });
    assert!(t[0].sw_update.is_none());

    // Asleep to park
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert!(t[0].state.is_some());
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_shift(ts, Some(P)), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to park after delay
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_with_shift(ts, Some(P)), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to drive
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_shift(ts, Some(D)), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to drive after delay
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_with_shift(ts, Some(D)), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to charging
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_charging(ts, 25), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to charging after delay
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_charging(ts, 25), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to offline
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(1).unwrap();
    let t = create_tables(&data_with_state(ts, Offline), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Asleep to offline after delay
    let start_time = chrono::Utc::now();
    let t = create_tables(&data_with_state(start_time, Asleep), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let prev_tables = &t[0];
    let ts = start_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = create_tables(&data_with_state(ts, Offline), prev_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(ts), end_date: None });
    assert!(t[1].sw_update.is_none());
}
