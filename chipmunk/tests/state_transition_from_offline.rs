use chipmunk::database::tables::{state::{State, StateStatus}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_charging, data_with_shift, data_with_state}, utils::ts_no_nanos, DELAYED_DATAPOINT_TIME_SEC};

pub mod common;

#[tokio::test]
async fn state_change_from_offline() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;
    chipmunk::init_log();

    // Offline to offline
    let offline_start_time = chrono::Utc::now();
    let t = chipmunk::logger::create_tables(&data_with_state(offline_start_time, Offline), &Tables::default(), car_id).await.unwrap();
    let offline_start_tables = &t[0];
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: None });
    assert!(t[0].sw_update.is_none());

    let offline_end_time = offline_start_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(offline_end_time, Offline), offline_start_tables, car_id).await.unwrap();
    let offline_end_tables = &t[0];
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());

    // Offline to offline after a delay
    let offline_end_time_1 = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(offline_end_time_1, Offline), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time_1)) });
    assert!(t[0].sw_update.is_none());

    // Offline to unknown
    let unknown_start_time = offline_end_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(unknown_start_time, Unknown), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Unknown, start_date: ts_no_nanos(unknown_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to unknown after a delay
    let unknown_start_time = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(unknown_start_time, Unknown), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Unknown, start_date: ts_no_nanos(unknown_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to park
    let parking_start_time = offline_end_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_shift(parking_start_time, Some(P)), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to park after a delay
    let parking_start_time = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_shift(parking_start_time, Some(P)), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Parked, start_date: ts_no_nanos(parking_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to drive
    let drive_start_time = offline_end_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_shift(drive_start_time, Some(R)), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(drive_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to drive after a delay
    let drive_start_time = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_shift(drive_start_time, Some(N)), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(drive_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to asleep
    let sleep_start_time = offline_end_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(sleep_start_time, Asleep), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(sleep_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to asleep after a delay
    let sleep_start_time = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_with_state(sleep_start_time, Asleep), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_none());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_none());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Asleep, start_date: ts_no_nanos(sleep_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to charging
    let charging_start_time = offline_end_time + Duration::try_seconds(1).unwrap();
    let t = chipmunk::logger::create_tables(&data_charging(charging_start_time, 25), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Offline to charging after a delay
    let charging_start_time = offline_end_time + Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let t = chipmunk::logger::create_tables(&data_charging(charging_start_time, 25), offline_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_none());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Offline, start_date: ts_no_nanos(offline_start_time), end_date: Some(ts_no_nanos(offline_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());
}