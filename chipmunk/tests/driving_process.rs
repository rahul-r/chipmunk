#![feature(async_closure)]
#![cfg_attr(rustfmt, rustfmt_skip)]

pub mod common;

use std::{
    sync::{Arc, Mutex},
    io::Write,
};

use chipmunk::{database::{tables::{
    address::Address,
    car::Car,
    charges::Charges,
    charging_process::ChargingProcess,
    drive::Drive,
    geofence::Geofence,
    position::Position,
    settings::Settings,
    state::{State, StateStatus},
    swupdate::SoftwareUpdate,
}, DBTable}, tasks};
use common::utils::{create_mock_osm_server, create_mock_tesla_server};
use rand::Rng;
use tesla_api::utils::miles_to_km;
use tesla_api::vehicle_data::{ShiftState, VehicleData};
use tokio::time::{sleep, Duration};

use crate::common::{test_data, utils::{ts_no_nanos, init_test_database}, DELAYED_DATAPOINT_TIME_SEC};

#[tokio::test]
pub async fn test_driving_and_parking() {
    use ShiftState::*;

    chipmunk::init_log();

    let random_http_port = rand::thread_rng().gen_range(4000..60000);
    std::env::set_var("HTTP_PORT", random_http_port.to_string());

    let _osm_mock = create_mock_osm_server().await;
    let pool = init_test_database("test_driving_and_parking").await;
    let env = chipmunk::load_env_vars().unwrap();

    // Make the logging period shorter to speed up the test
    let mut settings = Settings::db_get_last(&pool).await.unwrap();
    settings.logging_period_ms = 1;
    settings.db_insert(&pool).await.unwrap();

    // Set up a pointer to send vehicle data to the mock server
    let drive1_start_time = chrono::Utc::now();
    let data = test_data::data_with_shift(drive1_start_time, Some(D));
    let starting_odometer_mi = data.vehicle_state.as_ref().unwrap().odometer.unwrap();
    let data = Arc::new(Mutex::new(data));
    let send_response = Arc::new(Mutex::new(true));
    // Create a Tesla mock server
    let _tesla_mock = create_mock_tesla_server(data.clone(), send_response.clone()).await; // Assign the return value to a variable to keep the server alive

    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        panic_hook(panic_info);
        std::process::exit(1);
    }));

    let pool_clone = pool.clone();

    let _logger_task = tokio::task::spawn(async move {
        tasks::run(&env, &pool_clone).await.unwrap();
    });

    // Start driving
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 1);
    let drive1_start_address = Address::db_get_last(&pool).await.unwrap();
    assert!(drive1_start_time - drive1_start_address.inserted_at < chrono::Duration::try_seconds(2).unwrap());

    assert_eq!(Car::db_num_rows(&pool).await.unwrap(), 1);
    let car = Car::db_get_last(&pool).await.unwrap();

    assert_eq!(Charges::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);
    let state = State::db_get_last(&pool).await.unwrap();
    assert_eq!(state.state, StateStatus::Driving);
    assert_eq!(state.start_date, ts_no_nanos(drive1_start_time));

    assert_ne!(Position::db_num_rows(&pool).await.unwrap(), 0);
    let last_position = Position::db_get_last(&pool).await.unwrap();
    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 1);
    let drive = Drive::db_get_last(&pool).await.unwrap();

    assert_eq!(drive.car_id, car.id);
    assert_eq!(drive.start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(drive.end_date, None);
    assert_eq!(drive.start_address_id, Some(drive1_start_address.id as i32));
    assert_eq!(drive.end_address_id, None);
    assert!(drive.in_progress);
    assert_eq!(drive.end_km, last_position.odometer);
    assert_eq!(drive.distance, Some(0.0));
    assert_eq!(drive.duration_min, Some(0));
    assert_eq!(drive.start_position_id, Some(1));
    assert_eq!(drive.end_position_id, last_position.id);
    assert_eq!(drive.start_geofence_id, None);
    assert_eq!(drive.end_geofence_id, None);

    assert_eq!(last_position.date, Some(ts_no_nanos(drive1_start_time)));
    assert_eq!(last_position.drive_id, Some(drive.id));
    assert_eq!(last_position.car_id, car.id);

    assert_ne!(VehicleData::db_num_rows(&pool).await.unwrap(), 0);

    // Continue driving
    let timestamp = chrono::Utc::now();
    let odometer_mi = last_position.odometer.unwrap() + 123.4;
    let mut vehicle_data = test_data::data_with_shift(timestamp, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(Car::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(Charges::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);

    assert_ne!(Position::db_num_rows(&pool).await.unwrap(), 0);
    let last_position = Position::db_get_last(&pool).await.unwrap();
    assert_eq!(last_position.odometer, miles_to_km(&Some(odometer_mi)));
    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 1);
    let drive = Drive::db_get_last(&pool).await.unwrap();

    assert_eq!(drive.car_id, car.id);
    assert_eq!(drive.start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(drive.end_date, None);
    assert_eq!(drive.start_address_id, Some(drive1_start_address.id as i32));
    assert_eq!(drive.end_address_id, None);
    assert!(drive.in_progress);
    assert_eq!(drive.end_km, last_position.odometer);
    approx_eq!(drive.distance, miles_to_km(&Some(odometer_mi - starting_odometer_mi)));
    assert_eq!(drive.duration_min, Some(0));
    assert_eq!(drive.start_position_id, Some(1));
    assert_eq!(drive.end_position_id, last_position.id);
    assert_eq!(drive.start_geofence_id, None);
    assert_eq!(drive.end_geofence_id, None);

    assert_eq!(last_position.date, Some(ts_no_nanos(timestamp)));
    assert_eq!(last_position.drive_id, Some(drive.id));
    assert_eq!(last_position.car_id, car.id);

    let drive_duration_min = 9;
    let drive1_end_time = timestamp + chrono::Duration::try_minutes(drive_duration_min).unwrap();
    let mut vehicle_data = test_data::data_with_shift(drive1_end_time, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    let drive = Drive::db_get_last(&pool).await.unwrap();
    assert_eq!(drive.duration_min, Some(drive_duration_min as i16));

    // Make the time difference between the last position and the current time large enough to trigger a new drive
    // This is to test that the logger can handle a large time difference between the last and the current data points
    // This can happen if the there are no data points recorded while the car is parked
    // The logger should create a new drive with the same start and end address
    let drive1_num_positions = Position::db_num_rows(&pool).await.unwrap();
    let drive2_start_time = drive1_end_time + chrono::Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let mut vehicle_data = test_data::data_with_shift(drive2_start_time, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 2);
    let drive2_start_address = Address::db_get_last(&pool).await.unwrap();
    assert_eq!(Car::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(Charges::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 2);
    let states = State::db_get_all(&pool).await.unwrap();
    assert_eq!(states[0].state, StateStatus::Driving);
    assert_eq!(states[0].start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(states[0].end_date, Some(ts_no_nanos(drive1_end_time)));
    assert_eq!(states[0].car_id, car.id);
    assert_eq!(states[1].state, StateStatus::Driving);
    assert_eq!(states[1].start_date, ts_no_nanos(drive2_start_time));
    assert_eq!(states[1].end_date, Some(ts_no_nanos(drive2_start_time)));
    assert_eq!(states[1].car_id, car.id);

    let last_driving_position = Position::db_get_last(&pool).await.unwrap();
    assert_eq!(last_driving_position.odometer, miles_to_km(&Some(odometer_mi)));
    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 2);
    let drives = Drive::db_get_all(&pool).await.unwrap();
    assert_eq!(drives.len(), 2);
    let drive1 = &drives[0];
    let drive2 = &drives[1];

    assert_eq!(drive1.car_id, car.id);
    assert_eq!(drive1.start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(drive1.end_date, Some(ts_no_nanos(drive1_end_time)));
    assert_eq!(drive1.start_address_id, Some(drive1_start_address.id as i32));
    assert_eq!(drive1.end_address_id, Some(drive2_start_address.id as i32));
    assert!(!drive1.in_progress);
    assert_eq!(drive1.end_km, last_driving_position.odometer);
    approx_eq!(drive1.distance, miles_to_km(&Some(odometer_mi - starting_odometer_mi)));
    assert_eq!(drive1.start_position_id, Some(1));
    assert_eq!(drive1.end_position_id, Some(drive1_num_positions as i32));
    assert_eq!(drive1.start_geofence_id, None);
    assert_eq!(drive1.end_geofence_id, None);

    assert_eq!(drive2.car_id, car.id);
    assert_eq!(drive2.start_date, ts_no_nanos(drive2_start_time));
    assert_eq!(drive2.end_date, None);
    assert_eq!(drive2.start_address_id, Some(drive2_start_address.id as i32));
    assert_eq!(drive2.end_address_id, None);
    assert!(drive2.in_progress);
    assert_eq!(drive2.end_km, last_driving_position.odometer);
    approx_eq!(drive2.distance, miles_to_km(&Some(odometer_mi - starting_odometer_mi)));
    assert_eq!(drive2.start_position_id, Some(drive1_num_positions as i32 + 1));
    assert_eq!(drive2.end_position_id, last_driving_position.id);
    assert_eq!(drive2.start_geofence_id, None);
    assert_eq!(drive2.end_geofence_id, None);

    assert_eq!(last_driving_position.date, Some(ts_no_nanos(drive2_start_time)));
    assert_eq!(last_driving_position.drive_id, Some(drive2.id));
    assert_eq!(last_driving_position.car_id, car.id);

    // Stop driving / start park state
    let parking_start_time = drive1_end_time + chrono::Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let vehicle_data = test_data::data_with_shift(drive2_start_time, Some(P));
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 3);
    let address = Address::db_get_last(&pool).await.unwrap();
    assert_eq!(Car::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(Charges::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 3);
    let states = State::db_get_all(&pool).await.unwrap();
    assert_eq!(states[1].state, StateStatus::Driving);
    assert_eq!(states[1].start_date, ts_no_nanos(drive2_start_time));
    assert_eq!(states[1].end_date, Some(ts_no_nanos(drive2_start_time)));
    assert_eq!(states[1].car_id, car.id);
    assert_eq!(states[2].state, StateStatus::Parked);
    assert_eq!(states[2].start_date, ts_no_nanos(parking_start_time));
    assert_eq!(states[2].end_date, Some(ts_no_nanos(parking_start_time)));
    assert_eq!(states[2].car_id, car.id);

    let last_parking_position = Position::db_get_last(&pool).await.unwrap();
    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 2);
    let drive = Drive::db_get_last(&pool).await.unwrap();

    assert_eq!(drive.car_id, car.id);
    assert_eq!(drive.start_date, ts_no_nanos(drive2_start_time));
    assert_eq!(drive.end_date, Some(ts_no_nanos(drive2_start_time)));
    assert_eq!(drive.end_address_id, Some(address.id as i32));
    assert!(!drive.in_progress);
    assert_eq!(drive.end_km, last_driving_position.odometer);
    approx_eq!(drive.distance, miles_to_km(&Some(odometer_mi - starting_odometer_mi)));
    assert_eq!(drive.end_position_id, last_driving_position.id);
    assert_eq!(drive.start_geofence_id, None);
    assert_eq!(drive.end_geofence_id, None);

    assert!(last_parking_position.id > last_driving_position.id);
}