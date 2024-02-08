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
    use StateStatus::*;
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
    let ts_before_delayed_data = chrono::Utc::now().naive_utc();
    let odometer_mi = starting_odometer_mi + 123.4;
    let mut vehicle_data = test_data::data_with_shift(ts_before_delayed_data, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    vehicle_data.charge_state.as_mut().unwrap().battery_level = Some(49);
    let charge_start = Charges::from(&vehicle_data, 0);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    // Stop sending vehicle data
    *send_response.lock().unwrap() = false;
    wait_for_db!(pool);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 1);
    let state = State::db_get_last(&pool).await.unwrap();
    assert_eq!(state.state, StateStatus::Driving);
    let num_positions = Position::db_num_rows(&pool).await.unwrap();

    // Simulate charging without any recorded data point
    // 1. Create a driving data point after more than delayed data point threshold with the same odometer value
    // 2. Have battery level more than what it was in the previous data point. This will trigger a charging process
    // After this, a new charging process should be created and we should be in driving state (ends
    // current drive, create and finalize a charging state, and start a new drive)
    let ts_after_delayed_data = ts_before_delayed_data + chrono::Duration::seconds(DELAYED_DATAPOINT_TIME_SEC + 1);
    let mut vehicle_data = test_data::data_with_shift(ts_after_delayed_data, Some(D));
    vehicle_data.vehicle_state.as_mut().unwrap().odometer = Some(odometer_mi);
    vehicle_data.charge_state.as_mut().unwrap().battery_level = Some(55);
    let charge_end = Charges::from(&vehicle_data, 0);
    **data.lock().as_mut().unwrap() = vehicle_data;
    *send_response.lock().unwrap() = true;
    sleep(Duration::from_secs(1)).await; // Run the logger for some time
    // Stop sending vehicle data
    *send_response.lock().unwrap() = false;
    wait_for_db!(pool);

    assert_eq!(State::db_num_rows(&pool).await.unwrap(), 3);
    let states = State::db_get_all(&pool).await.unwrap();
    assert_eq!(states[0], State {id: 1, state: Driving, start_date: ts_no_nanos(drive1_start_time), end_date: Some(ts_no_nanos(ts_before_delayed_data)), car_id: 1 });
    assert_eq!(states[1], State {id: 2, state: Charging, start_date: ts_no_nanos(ts_before_delayed_data), end_date: Some(ts_no_nanos(ts_after_delayed_data)), car_id: 1 });
    assert_eq!(states[2], State {id: 3, state: Driving, start_date: ts_no_nanos(ts_after_delayed_data), end_date: Some(ts_no_nanos(ts_after_delayed_data)), car_id: 1 });

    assert_eq!(ChargingProcess::db_num_rows(&pool).await.unwrap(), 1);
    let cp = ChargingProcess::db_get_last(&pool).await.unwrap();
    let expected_cp = ChargingProcess {
        id: 1,
        start_date: ts_no_nanos(ts_before_delayed_data),
        end_date: Some(ts_no_nanos(ts_after_delayed_data)),
        charge_energy_added: charge_end.as_ref().map(|c| c.charge_energy_added).unwrap(),
        start_ideal_range_km: charge_start.as_ref().map(|c| c.ideal_battery_range_km).unwrap(),
        end_ideal_range_km: charge_end.as_ref().map(|c| c.ideal_battery_range_km).unwrap(),
        start_battery_level: charge_start.as_ref().map(|c| c.battery_level).unwrap(),
        end_battery_level: charge_end.as_ref().map(|c| c.battery_level).unwrap(),
        duration_min: Some((charge_end.as_ref().unwrap().date.unwrap() - charge_start.as_ref().unwrap().date.unwrap()).num_minutes() as i16),
        outside_temp_avg: Some((charge_start.as_ref().unwrap().outside_temp.unwrap() + charge_end.as_ref().unwrap().outside_temp.unwrap()) / 2.0),
        car_id: 1,
        position_id: num_positions as i32,
        address_id: Some(2),
        start_rated_range_km: charge_start.as_ref().unwrap().rated_battery_range_km,
        end_rated_range_km: charge_end.as_ref().unwrap().rated_battery_range_km,
        geofence_id: None,
        charge_energy_used: None,
        cost: None,
        charging_status: ChargeStat::Done,
    };
    assert_eq!(cp, expected_cp);

    assert_eq!(Charges::db_num_rows(&pool).await.unwrap(), 2);
    let charges = Charges::db_get_all(&pool).await.unwrap();
    let expected_charge_start = Charges {
        id: 1,
        charging_process_id: cp.id,
        ..charge_start.unwrap()
    };
    let expected_charge_end = Charges {
        id: 2,
        charging_process_id: cp.id,
        ..charge_end.unwrap()
    };
    assert_eq!(charges[0], expected_charge_start);
    assert_eq!(charges[1], expected_charge_end);

    assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 2);
    let drives = Drive::db_get_all(&pool).await.unwrap();
    assert_eq!(drives[0].start_date, ts_no_nanos(drive1_start_time));
    assert_eq!(drives[0].end_date, Some(ts_no_nanos(ts_before_delayed_data)));
    assert_eq!(drives[1].start_date, ts_no_nanos(ts_after_delayed_data));
    assert_eq!(drives[1].end_date, None);
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
