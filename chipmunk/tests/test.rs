#![feature(stmt_expr_attributes)]
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::too_many_lines)]

pub mod common;

use std::collections::VecDeque;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::sync::{Arc, Mutex};

use chipmunk::config::Config;
use chipmunk::database;
use chipmunk::database::tables::car::Car;
use chipmunk::database::tables::position::Position;
use chipmunk::database::tables::settings::Settings;
use chipmunk::database::tables::state::{StateStatus, State};
use chipmunk::DELAYED_DATAPOINT_TIME_SEC;
use rand::Rng;
use tesla_api::auth::AuthResponse;
use tokio::time::Duration;
use tokio::time::sleep;

use crate::common::test_data::data_with_shift;
use crate::common::utils::init_car_data_database;
use crate::common::utils::{create_drive_from_positions, create_mock_tesla_server, create_mock_tesla_server_vec, ts_no_nanos};
use crate::common::utils::{create_mock_osm_server, init_test_database};
use chipmunk::database::tables::drive::Drive;
use chipmunk::database::tables::Tables;
use chipmunk::database::DBTable;
use chipmunk::openstreetmap;
use chipmunk::task_data_processor::create_tables;
use common::test_data;
use tesla_api::vehicle_data::{DriveState, ShiftState, VehicleData};

#[rustfmt::skip]
pub fn create_drive_from_gpx() -> (Vec<VehicleData>, usize, usize) {
    let data = test_data::get_data(chrono::Utc::now());

    // Load gpx file
    let mut path = env::current_dir().unwrap();
    path.push("tests");
    path.push("common");
    path.push("route.gpx");
    let file = File::open(path).unwrap();
    let reader = BufReader::new(file);
    let gpx = gpx::read(reader).unwrap();

    let track: &gpx::Track = &gpx.tracks[0]; // Use the first track
    let segment: &gpx::TrackSegment = &track.segments[0]; // Use the first segment of the track

    let gpx_points = segment.points.iter();

    let mut data_points: Vec<VehicleData> = vec![];

    // Create few points in parked state before the drive
    let first_latitude = segment.points[0].point().y();
    let first_longitude = segment.points[0].point().x();
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
                latitude: Some(point.point().y()),
                longitude: Some(point.point().x()),
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

    // Create few points in parked state after the drive
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

#[tokio::test]
pub async fn check_vehicle_data() -> anyhow::Result<()> {
    // chipmunk::init_log();
    let random_http_port = rand::thread_rng().gen_range(4000..60000);
    std::env::set_var("HTTP_PORT", random_http_port.to_string());

    let pool = init_test_database("check_vehicle_data").await;
    let car_data_db_url = init_car_data_database("check_vehicle_data_car_data").await;
    std::env::set_var("CAR_DATA_DATABASE_URL", &car_data_db_url);
    let _osm_mock = create_mock_osm_server().await;

    // Make the logging period shorter to speed up the test
    let mut settings = Settings::db_get_last(&pool).await.unwrap();
    settings.logging_period_ms = 1;
    settings.db_insert(&pool).await.unwrap();

    let (vehicle_data_list, _drive_start_index, _drive_end_index) = create_drive_from_gpx();

    let vehicle_data_queue: VecDeque<VehicleData> = VecDeque::from(vehicle_data_list);
    let vehicle_data_queue_lock = Arc::new(Mutex::new(vehicle_data_queue));
    let _tesla_mock = create_mock_tesla_server_vec(vehicle_data_queue_lock.clone(), Arc::new(Mutex::new(true))).await; // Assign the return value to a variable to keep the server alive

    let mut config = Config::new(&pool).await;

    let pool_clone = pool.clone();
    let _logger_task = tokio::task::spawn(async move {
        if let Err(e) = chipmunk::tasks::run(&pool_clone, &mut config).await {
            log::error!("{e:?}");
        }
    });

    // Wait till all elements of the vehicle data queue are transferred
    loop {
        let l = vehicle_data_queue_lock.lock().unwrap().len();
        if l > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        } else {
            break;
        }
    }

    wait_for_db!(pool);

    let car_from_db = Car::db_get_all(&pool).await.unwrap();
    assert_eq!(car_from_db.len(), 1);
    assert_eq!(car_from_db[0].eid, 1234567890123456);
    assert_eq!(car_from_db[0].vid, 1);
    assert_eq!(car_from_db[0].vin, Some("EWABCD123UWE23456".into()));
    assert_eq!(car_from_db[0].name, Some("My Tesla".into()));
    assert_eq!(car_from_db[0].trim_badging, Some("74D".into()));
    assert_eq!(car_from_db[0].exterior_color, Some("Cherry".into()));
    assert_eq!(car_from_db[0].spoiler_type, Some("None".into()));
    assert_eq!(car_from_db[0].wheel_type, Some("Pinwheel20".into()));
    assert_eq!(car_from_db[0].marketing_name, Some("LR AWD".into()));

    let drive_from_db = Drive::db_get_last(&pool).await.unwrap();
    let positions = Position::db_get_for_drive(&pool, 1, drive_from_db.id).await.unwrap();
    let drive_calculated = create_drive_from_positions(&positions).unwrap();

    assert_eq!(drive_calculated.in_progress, drive_from_db.in_progress);
    assert!(drive_calculated.start_date - drive_from_db.start_date < chrono::Duration::try_seconds(1).unwrap());
    assert_eq!(drive_from_db.end_date.zip(drive_calculated.end_date).map(|(de, ee)| de - ee < chrono::Duration::try_seconds(1).unwrap()), Some(true));
    approx_eq!(drive_from_db.outside_temp_avg, drive_calculated.outside_temp_avg, 0.1);
    assert_eq!(drive_from_db.speed_max, drive_calculated.speed_max);
    assert_eq!(drive_from_db.power_max, drive_calculated.power_max);
    assert_eq!(drive_from_db.power_min, drive_calculated.power_min);
    assert_eq!(drive_from_db.start_ideal_range_km, drive_calculated.start_ideal_range_km);
    assert_eq!(drive_from_db.end_ideal_range_km, drive_calculated.end_ideal_range_km);
    assert_eq!(drive_from_db.start_km, drive_calculated.start_km);
    assert_eq!(drive_from_db.end_km, drive_calculated.end_km);
    approx_eq!(drive_from_db.distance, drive_calculated.distance);
    assert_eq!(drive_from_db.duration_min, drive_calculated.duration_min);
    assert_eq!(drive_from_db.car_id, drive_calculated.car_id);
    approx_eq!(drive_from_db.inside_temp_avg, drive_calculated.inside_temp_avg, 0.1);
    assert_eq!(drive_from_db.start_rated_range_km, drive_calculated.start_rated_range_km);
    assert_eq!(drive_from_db.end_rated_range_km, drive_calculated.end_rated_range_km);
    assert_eq!(drive_from_db.start_position_id, drive_calculated.start_position_id);
    assert_eq!(drive_from_db.end_position_id, drive_calculated.end_position_id);
    assert_eq!(drive_from_db.id, 1);
    assert_eq!(drive_from_db.start_address_id, Some(1));
    assert_eq!(drive_from_db.end_address_id, Some(2));
    assert_eq!(drive_from_db.start_geofence_id, None);
    assert_eq!(drive_from_db.end_geofence_id, None);

    //let car_data_database_url = std::env::var("CAR_DATA_DATABASE_URL").unwrap();
    let car_data_pool = database::initialize_car_data(&car_data_db_url).await.unwrap();
    assert_eq!(VehicleData::db_num_rows(&pool).await.unwrap(), VehicleData::db_num_rows(&car_data_pool).await.unwrap());
    Ok(())
}

// Test each request returns a response with different osm_id
#[tokio::test]
pub async fn test_osm_mock() {
    let _osm_mock = create_mock_osm_server().await;
    let client = openstreetmap::osm_client().unwrap();
    let res1 = openstreetmap::reverse_geocode(&client, &1.0, &1.0).await.unwrap();
    let res2 = openstreetmap::reverse_geocode(&client, &1.0, &1.0).await.unwrap();
    assert_ne!(res1.osm_id, res2.osm_id);
}

// Test vehicle data can be changed after creating the mock server
#[tokio::test]
pub async fn test_tesla_mock() {
    let data = test_data::get_data(chrono::Utc::now());
    let data = Arc::new(Mutex::new(data));

    let _tesla_mock = create_mock_tesla_server(data.clone(), Arc::new(Mutex::new(true))).await; // Assign the return value to a variable to keep the server alive

    let tokens = AuthResponse::default();
    let mut client = tesla_api::get_tesla_client(tokens, None).unwrap();

    data.lock().unwrap().drive_state.as_mut().unwrap().timestamp = Some(1234);
    let res1 = tesla_api::get_vehicle_data(&mut client, 1).await.unwrap();
    let vd1 = VehicleData::from_response_json(&res1).unwrap();

    data.lock().unwrap().drive_state.as_mut().unwrap().timestamp = Some(4321);
    let res2 = tesla_api::get_vehicle_data(&mut client, 1).await.unwrap();
    let vd2 = VehicleData::from_response_json(&res2).unwrap();

    assert_eq!(vd1.drive_state.unwrap().timestamp, Some(1234));
    assert_eq!(vd2.drive_state.unwrap().timestamp, Some(4321));
}

#[tokio::test]
async fn test_charged_and_driven_offline() {
    use ShiftState::*;
    use StateStatus::*;
    let car_id = 1i16;
    chipmunk::init_log();
    
    // Start from driving state
    let driving_start_time = chrono::Utc::now();
    let t = create_tables(&data_with_shift(driving_start_time, Some(D)), &Tables::default(), car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let drive_start_tables = &t[0];
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: None });

    let driving_intermediate_time = driving_start_time + chrono::Duration::try_seconds(5).unwrap();
    let t = create_tables(&data_with_shift(driving_intermediate_time, Some(D)), drive_start_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 1);
    let drive_tables = &t[0];

    // Create a data point after a delay with drive and charge data
    let driving_after_delay_time = driving_intermediate_time + chrono::Duration::try_seconds(DELAYED_DATAPOINT_TIME_SEC + 1).unwrap();
    let mut charged_and_driven_data = data_with_shift(driving_after_delay_time, Some(D));
    charged_and_driven_data.charge_state.as_mut().unwrap().battery_level = charged_and_driven_data.charge_state.as_ref().unwrap().battery_level.map(|mut c| {c += 10; c});
    let t = create_tables(&charged_and_driven_data, drive_tables, car_id).await.unwrap();
    assert_eq!(t.len(), 4); // 4 tables (1. end current drive, 2. charging process, 3. log charges, 4. start new drive)
    // Table 1: End current drive
    assert!(t[0].address.is_some());
    assert!(t[0].car.is_none());
    assert!(t[0].charges.is_none());
    assert!(t[0].charging_process.is_none());
    assert!(t[0].drive.is_some());
    assert!(t[0].position.is_some());
    assert!(t[0].settings.is_none());
    assert!(t[0].sw_update.is_none());
    assert!(t[0].state.is_some());
    assert_eq!(*t[0].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_start_time), end_date: Some(ts_no_nanos(driving_intermediate_time)) });
    // Table 2: Charging process
    assert!(t[1].address.is_some());
    assert!(t[1].car.is_none());
    assert!(t[1].charges.is_some());
    assert!(t[1].charging_process.is_some());
    assert!(t[1].drive.is_none());
    assert!(t[1].position.is_some());
    assert!(t[1].settings.is_none());
    assert!(t[1].sw_update.is_none());
    assert!(t[1].state.is_some());
    assert_eq!(*t[1].state.as_ref().unwrap(), State {car_id, id: 0, state: Charging, start_date: ts_no_nanos(driving_intermediate_time), end_date: Some(ts_no_nanos(driving_after_delay_time)) });
    // Table 3: Log charges
    assert!(t[2].address.is_none());
    assert!(t[2].car.is_none());
    assert!(t[2].charges.is_some());
    assert!(t[2].drive.is_none());
    assert!(t[2].position.is_none());
    assert!(t[2].settings.is_none());
    assert!(t[2].state.is_none());
    assert!(t[2].sw_update.is_none());
    // Table 4: Start new drive
    assert!(t[3].address.is_some());
    assert!(t[3].car.is_none());
    assert!(t[3].charges.is_none());
    assert!(t[3].charging_process.is_none());
    assert!(t[3].drive.is_some());
    assert!(t[3].position.is_some());
    assert!(t[3].settings.is_none());
    assert!(t[3].sw_update.is_none());
    assert!(t[3].state.is_some());
    assert_eq!(*t[3].state.as_ref().unwrap(), State {car_id, id: 0, state: Driving, start_date: ts_no_nanos(driving_after_delay_time), end_date: None });
}
