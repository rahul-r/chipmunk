#![feature(async_closure)]

pub mod common;

use std::{
    sync::{Arc, Mutex},
    io::Write,
};

use chipmunk::database::{tables::{
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
}, DBTable};
use chipmunk::database::types::ChargeStat;
use common::utils::{create_mock_osm_server, create_mock_tesla_server};
use rand::Rng;
use tesla_api::utils::miles_to_km;
use tesla_api::vehicle_data::ShiftState;
use tokio::time::{sleep, Duration};

use crate::common::{test_data, utils::{ts_no_nanos, init_test_database}, DELAYED_DATAPOINT_TIME_SEC};

#[tokio::test]
async fn test_hidden_charging_detection() {
    use ShiftState::*;
    // chipmunk::init_log();

    let random_http_port = rand::thread_rng().gen_range(4000..60000);
    std::env::set_var("HTTP_PORT", random_http_port.to_string());

    let _osm_mock = create_mock_osm_server();
    let pool = init_test_database("test_hidden_charging_detection").await;
    let env = chipmunk::load_env_vars().unwrap();

    // Make the logging period faster to speed up the test
    let mut settings = Settings::db_get_last(&pool).await.unwrap();
    settings.logging_period_ms = 1;
    settings.db_insert(&pool).await.unwrap();

    let drive1_start_time = chrono::Utc::now().naive_utc();
    let data = test_data::data_with_shift(drive1_start_time, Some(D));
    let starting_odometer_mi = data.vehicle_state.as_ref().unwrap().odometer.unwrap();
    let data = Arc::new(Mutex::new(data));
    let send_response = Arc::new(Mutex::new(true));
    // Create a Tesla mock server
    let _tesla_mock = create_mock_tesla_server(data.clone(), send_response.clone()); // Assign the return value to a variable to keep the server alive

    let pool_clone = pool.clone();

    let _logger_task = tokio::task::spawn(async move {
        chipmunk::logger::log(&pool_clone, &env).await.unwrap();
    });

    // Start driving
    sleep(Duration::from_secs(1)).await; // Run the logger for a second
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);
    let state = State::db_get_last(&pool).await.unwrap();
    assert_eq!(state.state, StateStatus::Driving);

    // Update the driving data point
    let drive1_timestamp1 = chrono::Utc::now().naive_utc();
    let odometer_mi = starting_odometer_mi + 123.4;
    let mut vehicle_data = test_data::data_with_shift(drive1_timestamp1, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    vehicle_data.charge_state.as_mut().unwrap().battery_level = Some(49);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);
    let state = State::db_get_last(&pool).await.unwrap();
    assert_eq!(state.state, StateStatus::Driving);

    // Simulate charging without any recorded data point
    // 1. Create a driving data point after more than delayed data point threshold with the same odometer value
    // 2. Have battery level more than what it was in the previous data point. This will trigger a charging process
    // After this, a new charging process should be created and we should be in driving state (ends
    // current drive, create and finalize a charging state, and start a new drive)
    let drive1_timestamp2 = drive1_timestamp1 + chrono::Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let mut vehicle_data = test_data::data_with_shift(drive1_timestamp2, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    vehicle_data.charge_state.as_mut().unwrap().battery_level = Some(55);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    // Stop sending vehicle data
    *send_response.lock().unwrap() = false;
    wait_for_db!(pool);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 3);
    let states = State::db_get_all(&pool).await.unwrap();
    assert_eq!(states[0].state, StateStatus::Driving);
    assert_eq!(states[0].start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(states[0].end_date, Some(ts_no_nanos(drive1_timestamp1)));
    assert_eq!(states[1].state, StateStatus::Charging);
    assert_eq!(states[1].start_date, ts_no_nanos(drive1_timestamp1));
    assert_eq!(states[1].end_date, Some(ts_no_nanos(drive1_timestamp2)));
    assert_eq!(states[2].state, StateStatus::Driving);
    assert_eq!(states[2].start_date, ts_no_nanos(drive1_timestamp2));
    assert_eq!(states[2].end_date, Some(ts_no_nanos(drive1_timestamp2)));
}

#[tokio::test]
async fn test_charging_process() {
    use ShiftState::*;
    // chipmunk::init_log();

    let random_http_port = rand::thread_rng().gen_range(4000..60000);
    std::env::set_var("HTTP_PORT", random_http_port.to_string());

    let _osm_mock = create_mock_osm_server();
    let pool = init_test_database("test_charging_process").await;
    let env = chipmunk::load_env_vars().unwrap();

    // Make the logging period faster to speed up the test
    let mut settings = Settings::db_get_last(&pool).await.unwrap();
    settings.logging_period_ms = 1;
    settings.db_insert(&pool).await.unwrap();

    // Setup a pointer to send vehicle data to the mock server
    let charging_start_time = chrono::Utc::now().naive_utc();
    let data = test_data::data_charging(charging_start_time, 25);
    let charge_state = data.charge_state.clone();
    let climate_state = data.climate_state.clone();
    let data = Arc::new(Mutex::new(data));
    let send_response = Arc::new(Mutex::new(true));
    // Create a Tesla mock server
    let _tesla_mock = create_mock_tesla_server(data.clone(), send_response.clone()); // Assign the return value to a variable to keep the server alive

    let pool_clone = pool.clone();
    let _logger_task = tokio::task::spawn(async move {
        chipmunk::logger::log(&pool_clone, &env).await.unwrap();
    });

    // Start charging
    sleep(Duration::from_secs(1)).await; // Run the logger for a second
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 1);
    let address = Address::db_get_last(&pool).await.unwrap();
    assert!(charging_start_time - address.inserted_at < chrono::Duration::seconds(2));

    assert_eq!(Car::db_num_rows(&pool).await.unwrap(), 1);
    let car = Car::db_get_last(&pool).await.unwrap();

    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);
    let state = State::db_get_last(&pool).await.unwrap();
    assert_eq!(state.state, StateStatus::Charging);
    assert_eq!(state.start_date, ts_no_nanos(charging_start_time));

    assert_ne!(Position::db_num_rows(&pool).await.unwrap(), 0);
    let last_position = Position::db_get_last(&pool).await.unwrap();

    assert_ne!(Charges::db_num_rows(&pool).await.unwrap(), 0);
    let charges = Charges::db_get_all(&pool).await.unwrap();
    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 1);
    let cp = ChargingProcess::db_get_last(&pool).await.unwrap();
    assert_eq!(cp.id, 1);
    assert_eq!(cp.start_date, ts_no_nanos(charging_start_time));
    assert_eq!(cp.end_date, last_position.date);
    assert_eq!(cp.charge_energy_added, charge_state.as_ref().unwrap().charge_energy_added);
    assert_eq!(cp.charge_energy_added, charges.last().unwrap().charge_energy_added);
    assert_eq!(cp.start_ideal_range_km, miles_to_km(&charge_state.as_ref().unwrap().ideal_battery_range));
    assert_eq!(cp.start_ideal_range_km, charges.first().unwrap().ideal_battery_range_km);
    assert_eq!(cp.end_ideal_range_km, miles_to_km(&charge_state.as_ref().unwrap().ideal_battery_range));
    assert_eq!(cp.end_ideal_range_km, charges.last().unwrap().ideal_battery_range_km);
    assert_eq!(cp.start_battery_level, charge_state.as_ref().unwrap().battery_level);
    assert_eq!(cp.start_battery_level, charges.first().unwrap().battery_level);
    assert_eq!(cp.end_battery_level, charge_state.as_ref().unwrap().battery_level);
    assert_eq!(cp.end_battery_level, charges.first().unwrap().battery_level);
    assert_eq!(cp.duration_min, Some(0));
    assert_eq!(cp.outside_temp_avg, climate_state.as_ref().unwrap().outside_temp); // We are not changing the temperature value of the test data. So the average will be the same as the current temperature
    assert_eq!(cp.car_id, car.id);
    assert_eq!(cp.position_id, 1); // This will be the id of the first position row
    assert_eq!(cp.address_id, Some(address.id as i32));
    assert_eq!(cp.start_rated_range_km, miles_to_km(&charge_state.as_ref().unwrap().battery_range));
    assert_eq!(cp.start_rated_range_km, charges.first().unwrap().rated_battery_range_km);
    assert_eq!(cp.end_rated_range_km, miles_to_km(&charge_state.as_ref().unwrap().battery_range));
    assert_eq!(cp.end_rated_range_km, charges.last().unwrap().rated_battery_range_km);
    assert_eq!(cp.geofence_id, None);
    assert_eq!(cp.charge_energy_used, None);
    assert_eq!(cp.cost, None);
    assert_eq!(cp.charging_status, ChargeStat::Charging);

    // Stop charging and start parked state
    let charging_end_time1 = Position::db_get_last(&pool).await.unwrap().date;
    let charging_end_time2 = Charges::db_get_last(&pool).await.unwrap().date;
    assert_eq!(charging_end_time1, charging_end_time2);
    let parking_start_time = chrono::Utc::now().naive_utc();
    let parked_data = test_data::data_with_shift(parking_start_time, Some(P));
    **data.lock().as_mut().unwrap() = parked_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    *send_response.lock().unwrap() = false; // Tell the mock server to stop sending vehicle data
    wait_for_db!(pool);

    assert_eq!(Address::db_num_rows(&pool).await.unwrap(), 1);
    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Geofence::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(SoftwareUpdate::db_num_rows(&pool).await.unwrap(), 0);
    assert_eq!(Settings::db_num_rows(&pool).await.unwrap(), 1);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 2);
    let states = State::db_get_all(&pool).await.unwrap();
    assert_eq!(states[0].state, StateStatus::Charging);
    assert_eq!(states[0].start_date, ts_no_nanos(charging_start_time));
    assert_eq!(states[0].end_date, charging_end_time1);
    assert_eq!(states[1].state, StateStatus::Parked);
    assert_eq!(states[1].start_date, ts_no_nanos(parking_start_time));
    assert_eq!(states[1].end_date, Some(ts_no_nanos(parking_start_time)));
}

// * Test hidden charging process
// Create a drive
// Wait for for more than the threshold
// Create new data point with more battery level than before and with the same odometer
// Check that a charging process is created in database
// Check that two charges datapoints are created (one from previous data point and one from the current data point) in database
