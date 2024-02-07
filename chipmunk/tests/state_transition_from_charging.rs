pub mod common;

use chipmunk::database::tables::{state::{StateStatus, State}, Tables};
use chrono::Duration;
use tesla_api::vehicle_data::ShiftState;

use crate::common::{test_data::{data_with_state, data_with_shift, data_charging}, DELAYED_DATAPOINT_TIME_SEC, utils::ts_no_nanos};


#[tokio::test]
async fn state_change_from_charging() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;
    chipmunk::init_log();

    // Charging to charging
    let charging_start_time = chrono::Utc::now().naive_utc();
    let t = chipmunk::logger::create_tables(&data_charging(charging_start_time), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let charging_start_tables = &t[0];
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: None });
    assert!(t[0].sw_update.is_none());

    let charging_end_time = charging_start_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_charging(charging_end_time), charging_start_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let charging_end_tables = &t[0];
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
    assert!(t[0].sw_update.is_none());

    // Charging to charging after a delay
    let charging_end_time_1 = charging_start_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let t = chipmunk::logger::create_tables(&data_charging(charging_end_time_1), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time_1)) });
    assert!(t[0].sw_update.is_none());

    // Charging to park
    let parking_start_time = charging_end_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_with_shift(parking_start_time, Some(P)), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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

    // Charging to park after a delay
    let parking_start_time = charging_end_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    // dbg!(&charging_end_tables.state);
    let t = chipmunk::logger::create_tables(&data_with_shift(parking_start_time, Some(P)), charging_end_tables, car_id).await.unwrap();
    // dbg!(&t[0].state);
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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

    // Charging to drive
    let driving_start_time = charging_end_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_with_shift(driving_start_time, Some(D)), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Charging to drive after a delay
    let driving_start_time = charging_end_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let t = chipmunk::logger::create_tables(&data_with_shift(driving_start_time, Some(D)), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
    assert!(t[0].sw_update.is_none());
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_none());
    assert!(t[1].charging_process.is_none());
    assert!(t[1].drive.is_some());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: None });
    assert!(t[1].sw_update.is_none());

    // Charging to asleep
    let sleep_start_time = charging_end_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_with_state(sleep_start_time, Asleep), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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

    // Charging to asleep after a delay
    let sleep_start_time = charging_end_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let t = chipmunk::logger::create_tables(&data_with_state(sleep_start_time, Asleep), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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

    // Charging to offline
    let offline_start_time = charging_end_time + Duration::seconds(1);
    let t = chipmunk::logger::create_tables(&data_with_state(offline_start_time, Offline), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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

    // Charging to offline after a delay
    let offline_start_time = charging_end_time + Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let t = chipmunk::logger::create_tables(&data_with_state(offline_start_time, Offline), charging_end_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 2);
    assert!(t[0].address.is_none());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_some());
    assert!(t[0].charging_process.is_some());
    assert!(t[0].drive.is_none());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(charging_start_time), end_date: Some(ts_no_nanos(charging_end_time)) });
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
}