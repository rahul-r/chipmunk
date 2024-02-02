#![feature(stmt_expr_attributes)]

pub mod common;

use chipmunk::database::types::DriveStatus;
use chipmunk::database::DBTable;
use chrono::Duration;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};

use chipmunk::database::tables::drive::Drive;
use chipmunk::database::tables::state::{State, StateStatus};
use chipmunk::database::tables::Tables;
use chipmunk::{database, openstreetmap};
use rand::Rng;
use tesla_api::utils::{miles_to_km, mph_to_kmh, timestamp_to_naivedatetime};
use tesla_api::vehicle_data::{DriveState, ShiftState, VehicleData};

use common::test_data::{self, data_with_shift};
use common::utils::ts_no_nanos;

use crate::common::DELAYED_DATAPOINT_TIME_SEC;
use crate::common::utils::create_mock_tesla_server;
use crate::common::utils::{create_mock_osm_server, init_test_database};

#[rustfmt::skip]
pub fn create_drive_from_gpx() -> (Vec<VehicleData>, usize, usize) {
    let data = test_data::get_data(chrono::Utc::now().naive_utc());

    // Load gpx file
    let path = Path::new("/chipmunk/chipmunk/tests/route.gpx");
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let gpx = gpx::read(reader).unwrap();

    let track: &gpx::Track = &gpx.tracks[0]; // Use the first track
    let segment: &gpx::TrackSegment = &track.segments[0]; // Use the first segment of the track

    let gpx_points = segment.points.iter();

    let mut data_points: Vec<VehicleData> = vec![];

    // Create few points in parked state before the drive
    let first_latitude = segment.points[0].point().y() as f32;
    let first_longitude = segment.points[0].point().x() as f32;
    for _ in 0..10 {
        data_points.push(VehicleData {
            drive_state: Some(DriveState {
                latitude: Some(first_latitude),
                longitude: Some(first_longitude),
                shift_state: Some(ShiftState::P),
                timestamp: data_points
                            .last()
                            .and_then(|d| d.timestamp_epoch()) // Get timestamp
                            .map(|t| t + 1000) // Increment timestamp by 1 second
                            .or_else(|| Some(chrono::Utc::now().timestamp_millis() as u64)), // If timestamp is None, use current timestamp
                ..data.drive_state.clone().unwrap()
            }),
            ..data.clone()
        });
    }

    // Next index of the first drive point
    let drive_start_index = data_points.len();

    for (_index, point) in gpx_points.enumerate() {
        if _index % 3 != 0 { // Use every third point and skip the rest to reduce the number of points
            continue;
        }

        // Increment the timestamp by 1 second for each point
        let timestamp = data_points.last().unwrap().drive_state.as_ref().unwrap().timestamp.map(|t| t + 1000);

        let vehicle_data = VehicleData {
            drive_state: Some(DriveState {
                latitude: Some(point.point().y() as f32),
                longitude: Some(point.point().x() as f32),
                shift_state: Some(ShiftState::D),
                timestamp,
                ..data.drive_state.clone().unwrap()
            }),
            ..data.clone()
        };
        data_points.push(vehicle_data.clone());
    }

    // Next index of the first parked point after the drive, this is also the last point of the drive
    let drive_end_index = data_points.len();

    // Create few ploints in parked state after the drive
    let last_latitude = data_points.last().unwrap().drive_state.as_ref().unwrap().latitude;
    let last_longitude = data_points.last().unwrap().drive_state.as_ref().unwrap().longitude;
    for _ in 0..10 {
        data_points.push(VehicleData {
            drive_state: Some(DriveState {
                latitude: last_latitude,
                longitude: last_longitude,
                shift_state: Some(ShiftState::P),
                timestamp: data_points.last().unwrap().drive_state.as_ref().unwrap().timestamp.map(|t| t + 1000),
                ..data.drive_state.clone().unwrap()
            }),
            ..data.clone()
        });
    }

    (data_points, drive_start_index, drive_end_index)
}

#[rustfmt::skip]
fn calculate_expected_drive(
    vehicle_data_list: &[VehicleData],
    drive_start_index: usize,
    drive_end_index: usize,
    car_id: i16,
) -> Drive {
    let first_drive_data = vehicle_data_list[drive_start_index].clone();
    let last_drive_data = vehicle_data_list[drive_end_index - 1].clone();

    let mut outside_tmp_total = 0f32;
    let mut inside_temp_total = 0f32;
    let mut speed_max = 0f32;
    let mut power_max = -9999f32;
    let mut power_min = 9999f32;

    for data in vehicle_data_list.iter().take(drive_end_index + 1).skip(drive_start_index) {
        outside_tmp_total += data.climate_state.as_ref().unwrap().outside_temp.unwrap();
        inside_temp_total += data.climate_state.as_ref().unwrap().inside_temp.unwrap();
        speed_max = speed_max.max(mph_to_kmh(&data.drive_state.as_ref().unwrap().speed).unwrap());
        power_max = power_max.max(data.drive_state.as_ref().unwrap().power.unwrap());
        power_min = power_min.min(data.drive_state.as_ref().unwrap().power.unwrap());
    }

    let num_drive_points = drive_end_index - drive_start_index + 1;

    let start_date = timestamp_to_naivedatetime(first_drive_data.drive_state.as_ref().unwrap().timestamp).unwrap();
    let _end_date = timestamp_to_naivedatetime(last_drive_data.drive_state.as_ref().unwrap().timestamp).unwrap();
    let duration_min = (_end_date - start_date).num_minutes();
    let start_km = miles_to_km(&first_drive_data.vehicle_state.map(|c| c.odometer).unwrap()).unwrap();
    let end_km = miles_to_km(&last_drive_data.vehicle_state.map(|c| c.odometer).unwrap()).unwrap();

    let end_date;
    let end_address_id;
    let status;
    let end_position_id;
    if vehicle_data_list.last().unwrap().is_driving() {
        end_date = None;
        end_address_id = None;
        status = DriveStatus::Driving;
        end_position_id = None;
    } else {
        end_date = Some(_end_date);
        end_address_id = Some(2);
        status = DriveStatus::NotDriving;
        end_position_id = Some(drive_end_index as i32);
    }

    Drive {
        id: 0,
        start_date,
        end_date,
        outside_temp_avg: Some(outside_tmp_total / num_drive_points as f32),
        speed_max: Some(speed_max),
        power_max: Some(power_max),
        power_min: Some(power_min),
        start_ideal_range_km: miles_to_km(&first_drive_data.charge_state.clone().map(|c| c.ideal_battery_range).unwrap()),
        end_ideal_range_km: miles_to_km(&last_drive_data.charge_state.clone().map(|c| c.ideal_battery_range).unwrap()),
        start_km: Some(start_km),
        end_km: Some(end_km),
        distance: Some(end_km - start_km),
        duration_min: Some(duration_min as i16),
        car_id,
        inside_temp_avg: Some(inside_temp_total / num_drive_points as f32),
        start_rated_range_km: miles_to_km(&first_drive_data.charge_state.map(|c| c.battery_range).unwrap()),
        end_rated_range_km: miles_to_km(&last_drive_data.charge_state.map(|c| c.battery_range).unwrap()),
        start_address_id: Some(1),
        end_address_id,
        start_position_id: Some(drive_start_index as i32 + 1),
        end_position_id,
        status,
        start_geofence_id: None,
        end_geofence_id: None,
    }
}

#[tokio::test]
async fn check_vehicle_data() -> anyhow::Result<()> {
    let random_http_port = rand::thread_rng().gen_range(4000..60000);
    std::env::set_var("HTTP_PORT", random_http_port.to_string());

    let pool = init_test_database("check_vehicle_data").await;
    let _osm_mock = create_mock_osm_server();

    let (vehicle_data_list, drive_start_index, drive_end_index) = create_drive_from_gpx();
    let mut vin_id_map = database::tables::car::get_vin_id_map(&pool).await;
    let mut tables = Tables::default();

    for data in &vehicle_data_list {
        (vin_id_map, tables) =
            chipmunk::logger::process_vehicle_data(&pool, vin_id_map, tables, data).await;
    }

    let expected_drive = calculate_expected_drive(
        &vehicle_data_list,
        drive_start_index,
        drive_end_index,
        1i16,
    );
    
    #[rustfmt::skip]
    {
        assert_eq!(Drive::db_num_rows(&pool).await.unwrap(), 1);
        let last_row = Drive::db_get_last(&pool).await.unwrap();
        assert_eq!(last_row.start_date, expected_drive.start_date);
        assert_eq!(last_row.end_date, expected_drive.end_date);
        approx_eq!(last_row.outside_temp_avg, expected_drive.outside_temp_avg);
        assert_eq!(last_row.speed_max, expected_drive.speed_max);
        assert_eq!(last_row.power_max, expected_drive.power_max);
        assert_eq!(last_row.power_min, expected_drive.power_min);
        assert_eq!(last_row.start_ideal_range_km, expected_drive.start_ideal_range_km);
        assert_eq!(last_row.end_ideal_range_km, expected_drive.end_ideal_range_km);
        assert_eq!(last_row.start_km, expected_drive.start_km);
        assert_eq!(last_row.end_km, expected_drive.end_km);
        assert_eq!(last_row.distance, expected_drive.distance);
        assert_eq!(last_row.duration_min, expected_drive.duration_min);
        assert_eq!(last_row.car_id, expected_drive.car_id);
        approx_eq!(last_row.inside_temp_avg, expected_drive.inside_temp_avg);
        assert_eq!(last_row.start_address_id, expected_drive.start_address_id);
        assert_eq!(last_row.end_address_id, expected_drive.end_address_id);
        assert_eq!(last_row.start_rated_range_km, expected_drive.start_rated_range_km);
        assert_eq!(last_row.end_rated_range_km, expected_drive.end_rated_range_km);
        assert_eq!(last_row.start_position_id, expected_drive.start_position_id);
        assert_eq!(last_row.end_position_id, expected_drive.end_position_id);
        assert_eq!(last_row.start_geofence_id, expected_drive.start_geofence_id);
        assert_eq!(last_row.end_geofence_id, expected_drive.end_geofence_id);
        assert_eq!(last_row.status, expected_drive.status);
    }
    Ok(())
}

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
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
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
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
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
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
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
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
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
    assert_eq!(t[0].charges.as_ref().unwrap().id, 0);
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(ts), end_date: None });

    // TODO: Test starting when a charging session is active
}

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
}

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
}

#[tokio::test]
async fn state_change_from_offline() {
    // TODO:
    // Offline to offline
    // Offline to park
    // Offline to drive
    // Offline to charging
    // Offline to asleep
}

#[tokio::test]
async fn state_change_from_charging() {
    // TODO:
    // Charging to charging
    // Charging to park
    // Charging to drive
    // Charging to asleep
    // Charging to offline
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

    // Create another driving data point in 10 minutes
    let second_ts = first_ts + Duration::minutes(DELAYED_DATAPOINT_TIME_SEC - 1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states when driving data points are 10 minutes apart
    assert_eq!(second_data_point.len(), 1);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(second_ts)) });

    // Create another driving data point after 11 minutes
    let second_ts = first_ts + Duration::minutes(DELAYED_DATAPOINT_TIME_SEC);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states when driving data points are more than 10 minutes apart
    assert_eq!(second_data_point.len(), 2);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(first_ts)) });
    assert_eq!(*second_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(second_ts), end_date: None });

    // Create another driving data point after 10 minutes (10 minutes and 1 second)
    let second_ts = first_ts  + Duration::minutes((DELAYED_DATAPOINT_TIME_SEC - 1) * 60 + 1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    // Verify the states when driving data points are more than 10 minutes apart
    assert_eq!(second_data_point.len(), 2);
    assert_eq!(*second_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(first_ts)) });
    assert_eq!(*second_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(second_ts), end_date: None });

    // Test state changes when a data point is received after 10 minutes of driving and the car hasn't moved
    let second_ts = first_ts + Duration::minutes(1);
    let second_data_point = chipmunk::logger::create_tables(&data_with_shift(second_ts, Some(D)), &first_data_point[0], car_id).await.unwrap();
    let third_ts = second_ts + Duration::minutes(1);
    let third_data_point = chipmunk::logger::create_tables(&data_with_shift(third_ts, Some(D)), &second_data_point[0], car_id).await.unwrap();
    let fourth_ts = third_ts + Duration::minutes(DELAYED_DATAPOINT_TIME_SEC);
    let fourth_data_point = chipmunk::logger::create_tables(&data_with_shift(fourth_ts, Some(D)), &third_data_point[0], car_id).await.unwrap();
    // Verify the states
    assert_eq!(second_data_point.len(), 1);
    assert_eq!(third_data_point.len(), 1);
    assert_eq!(fourth_data_point.len(), 2);
    // Check for end of first drive
    assert_eq!(*fourth_data_point[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(first_ts), end_date: Some(ts_no_nanos(third_ts)) });
    // Check for start of new drive
    assert_eq!(*fourth_data_point[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(fourth_ts), end_date: None });

    // TODO: Create tests to check charging states
}

// Test each request returns a response with different osm_id
#[tokio::test]
async fn test_osm_mock() {
    let _osm_mock = create_mock_osm_server();
    let client = openstreetmap::osm_client().unwrap();
    let res1 = openstreetmap::reverse_geocode(&client, &1.0, &1.0).await.unwrap();
    let res2 = openstreetmap::reverse_geocode(&client, &1.0, &1.0).await.unwrap();
    assert_ne!(res1.osm_id, res2.osm_id);
}

// Test vehicle data can be changed after creating the mock server
#[tokio::test]
async fn test_tesla_mock() {
    let data = test_data::get_data(chrono::Utc::now().naive_utc());
    let data = Arc::new(Mutex::new(data));

    let _tesla_mock = create_mock_tesla_server(data.clone(), Arc::new(Mutex::new(true))); // Assign the return value to a variable to keep the server alive

    let client = tesla_api::get_tesla_client("").unwrap();

    data.lock().unwrap().drive_state.as_mut().unwrap().timestamp = Some(1234);
    let res1 = tesla_api::get_vehicle_data(&client, 1).await.unwrap();
    let vd1 = VehicleData::from_response_json(&res1).unwrap();

    data.lock().unwrap().drive_state.as_mut().unwrap().timestamp = Some(4321);
    let res2 = tesla_api::get_vehicle_data(&client, 1).await.unwrap();
    let vd2 = VehicleData::from_response_json(&res2).unwrap();

    assert_eq!(vd1.drive_state.unwrap().timestamp, Some(1234));
    assert_eq!(vd2.drive_state.unwrap().timestamp, Some(4321));
}
