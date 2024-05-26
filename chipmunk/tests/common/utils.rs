#![cfg_attr(rustfmt, rustfmt_skip)]

use std::{collections::VecDeque, sync::{Arc, Mutex}};

use rand::Rng;

use chipmunk::database::{self, tables::{charges::Charges, charging_process::ChargingProcess, drive::Drive, position::Position, token::Token}, types::ChargeStat};
use chrono::{DateTime, Utc};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use chipmunk::charging::calculate_energy_used;
use tesla_api::{auth::AuthResponse, vehicle_data::{VehicleData, Vehicles}};

/// Asserts that two floats are approximately equal.
#[macro_export]
macro_rules! approx_eq {
    ($x:expr, $y:expr) => {{
        approx_eq!($x, $y, 0.01);
    }};

    ($x:expr, $y:expr, $accuracy:expr) => {{
        if !($x.is_none() && $y.is_none()) { // Don't panic if both values are None
            let res = $x.zip($y).map(|(a, b)| a.abs() - b.abs());
            if res.is_none() || res > Some($accuracy) {
                panic!(
                    "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                    $x, $y
                );
            }
        }
    }};
}

#[macro_export]
macro_rules! wait_for_db {
    ($x:expr) => {
        print!("Waiting for database transactions to complete");
        while $x.num_idle() as u32 != $x.size() {
            print!(".");
            std::io::stdout().flush().unwrap();
            sleep(Duration::from_secs(1)).await;
        }
        println!();
    };
}

#[derive(Serialize, Deserialize)]
struct ApiResponse<T> {
    response: Option<T>,
    // error: Option<String>,
    // error_description: Option<String>,
    // messages: Option<serde_json::Value>, // format -> {"field1":["problem1","problem2"],...}
}

fn create_vehicles_response_json() -> String {
    let vehicle = Vehicles {
        id: Some(1),
        vehicle_id: Some(1234),
        ..Vehicles::default()
    };
    let vehicles = vec![vehicle];
    let resp = ApiResponse {
        response: Some(vehicles),
    };
    serde_json::to_string(&resp).unwrap()
}

pub async fn create_mock_tesla_server(vehicle_data: Arc<Mutex<VehicleData>>, send_response: Arc<Mutex<bool>>) -> mockito::ServerGuard {
    // Request a new server from the pool
    let mut server = mockito::Server::new_async().await;

    let vehicle_data_response = move |w: &mut dyn std::io::Write| {
        if *send_response.lock().unwrap() {
            let resp = ApiResponse {
                response: Some(vehicle_data.lock().unwrap().clone()),
            };
            let resp_json = serde_json::to_string(&resp).unwrap();

            w.write_all(resp_json.as_bytes()).unwrap();
        } else {
            w.write_all("chipmunk_test_in_progress".as_bytes()).unwrap();
        }
        Ok(())
    };

    let srv1 = server
        .mock("GET", "/vehicles/1/vehicle_data")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_chunked_body(vehicle_data_response)
        .create_async();

    let srv2 = server
        .mock("GET", "/products")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_vehicles_response_json())
        .create_async();

    let tokens = AuthResponse {
        access_token: "access_token".to_string(),
        refresh_token: "refresh_token".to_string(),
        id_token: "id_token".to_string(),
        expires_in: 6000,
        token_type: "Bearer".to_string(),
    };

    let srv3 = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&tokens).unwrap())
        .create_async();

    let (_srv1, _srv2, _srv3) = futures::join!(srv1, srv2, srv3);

    let mock_url = server.url();
    std::env::set_var("MOCK_TESLA_BASE_URL", mock_url);
    server
}

pub async fn create_mock_tesla_server_vec(vehicle_data: Arc<Mutex<VecDeque<VehicleData>>>, send_response: Arc<Mutex<bool>>) -> mockito::ServerGuard {
    // Request a new server from the pool
    let mut server = mockito::Server::new_async().await;

    let vehicle_data_response = move |w: &mut dyn std::io::Write| {
        if *send_response.lock().unwrap() {
            let r = vehicle_data.lock().unwrap().pop_front().clone();
            if r.is_some() {
                let resp = ApiResponse {
                    response: Some(r),
                };
                let resp_json = serde_json::to_string(&resp).unwrap();

                w.write_all(resp_json.as_bytes()).unwrap();
            } else {
                w.write_all("chipmunk_test_in_progress".as_bytes()).unwrap();
            }
        } else {
            w.write_all("chipmunk_test_in_progress".as_bytes()).unwrap();
        }
        Ok(())
    };

    let srv1 = server
        .mock("GET", "/vehicles/1/vehicle_data")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_chunked_body(vehicle_data_response)
        .create_async();

    let srv2 = server
        .mock("GET", "/products")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_vehicles_response_json())
        .create_async();

    let tokens = AuthResponse {
        access_token: "access_token".to_string(),
        refresh_token: "refresh_token".to_string(),
        id_token: "id_token".to_string(),
        expires_in: 6000,
        token_type: "Bearer".to_string(),
    };

    let srv3 = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(serde_json::to_string(&tokens).unwrap())
        .create_async();

    let (_srv1, _srv2, _srv3) = futures::join!(srv1, srv2, srv3);

    let mock_url = server.url();
    std::env::set_var("MOCK_TESLA_BASE_URL", mock_url);
    server
}

pub async fn create_mock_osm_server() -> mockito::ServerGuard {
    // Request a new server from the pool
    let mut server = mockito::Server::new_async().await;

    let osm_response_string = std::fs::read_to_string("tests/common/osm_response.json").unwrap();
    let resp_json: chipmunk::openstreetmap::OsmResponse =
        serde_json::from_str(&osm_response_string).unwrap();
    let resp_arc = Arc::new(Mutex::new(resp_json));

    let osm_response = move |w: &mut dyn std::io::Write| {
        resp_arc.lock().unwrap().osm_id = Some(rand::thread_rng().gen_range(1..9999999)); // make osm_id random
        let resp_json = serde_json::to_string(&resp_arc.lock().unwrap().clone()).unwrap();
        w.write_all(resp_json.as_bytes()).unwrap();
        Ok(())
    };

    // Create a mock
    let _mock = server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_chunked_body(osm_response)
        .create_async()
        .await;
    let mock_url = server.url();
    std::env::set_var("MOCK_OSM_BASE_URL", mock_url);
    server
}

pub async fn init_test_database(db_name: &str) -> sqlx::Pool<sqlx::Postgres> {
    dotenvy::dotenv().ok();
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TEST_DATABASE_URL`");
    let mut parsed_url = Url::parse(&url).unwrap();
    let username = parsed_url.username();

    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query(format!("DROP DATABASE IF EXISTS {db_name}").as_str())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(format!("CREATE DATABASE {db_name} OWNER={username}").as_str())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    parsed_url.set_path(db_name);
    let new_url = parsed_url.as_str();

    let pool = database::initialize(new_url).await.unwrap();
    // delete all entries from database tables before running tests
    sqlx::query!("TRUNCATE TABLE cars, drives, positions, addresses, settings, states, charges, charging_processes RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    database::tables::initialize(&pool).await.unwrap();

    let tokens = AuthResponse {
        access_token: "access_token".to_string(),
        refresh_token: "refresh_token".to_string(),
        id_token: "id_token".to_string(),
        expires_in: 6000,
        token_type: "Bearer".to_string(),
    };
    let encryption_key = "secret password acbdefghijklmnop";
    std::env::set_var("TOKEN_ENCRYPTION_KEY", encryption_key);
    Token::db_insert(&pool, &tokens, encryption_key)
        .await
        .unwrap();

    pool
}

pub async fn init_car_data_database(db_name: &str) -> String {
    dotenvy::dotenv().ok();
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TEST_DATABASE_URL`");
    let mut parsed_url = Url::parse(&url).unwrap();
    let username = parsed_url.username();

    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query(format!("DROP DATABASE IF EXISTS {db_name}").as_str())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(format!("CREATE DATABASE {db_name} OWNER={username}").as_str())
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    parsed_url.set_path(db_name);
    let new_url = parsed_url.as_str();

    let pool = database::initialize(new_url).await.unwrap();
    // delete all entries from database tables before running tests
    sqlx::query!("TRUNCATE TABLE car_data RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;

    new_url.into()
}

pub fn ts_no_nanos(ts: DateTime<Utc>) -> DateTime<Utc> {
    let timestamp = ts.timestamp_millis();
    let secs = timestamp / 1000;
    let nsecs = (timestamp % 1000 * 1_000_000) as u32;
    DateTime::from_timestamp(secs, nsecs).unwrap()
}

pub fn create_drive_from_positions(positions: &[Position]) -> Option<Drive> {
    let start_position = positions.first()?;
    let end_position = positions.last()?;

    let filtered_outside_temp = positions.iter().filter_map(|p| p.outside_temp).collect::<Vec<_>>();
    let filtered_inside_temp = positions.iter().filter_map(|p| p.inside_temp).collect::<Vec<_>>();

    Some(Drive {
        id: 0,
        in_progress: false,
        start_date: start_position.date.unwrap_or_default(),
        end_date: end_position.date,
        outside_temp_avg: Some(filtered_outside_temp.iter().sum::<f32>() / filtered_outside_temp.len() as f32),
        speed_max: positions.iter().filter_map(|p| p.speed).map(|v| v as i32).max().map(|speed| speed as f32),
        power_max: positions.iter().filter_map(|p| p.power).map(|v| v as i32).max().map(|power| power as f32),
        power_min: positions.iter().filter_map(|p| p.power).map(|v| v as i32).min().map(|power| power as f32),
        start_ideal_range_km: positions.iter().find_map(|p| p.ideal_battery_range_km), // Take the first non None value
        end_ideal_range_km: positions.iter().filter_map(|p| p.ideal_battery_range_km).last(), // Take the last non None value
        start_km: start_position.odometer,
        end_km: end_position.odometer,
        distance: end_position.odometer.zip(start_position.odometer).map(|(end, start)| end - start),
        duration_min: end_position.date.zip(start_position.date).map(|(e, s)| e - s).map(|d| (d.num_seconds() as f64 / 60.0).round() as i16),
        car_id: start_position.car_id,
        inside_temp_avg: Some(filtered_inside_temp.iter().sum::<f32>() / filtered_inside_temp.len() as f32),
        start_address_id: None,
        end_address_id: None,
        start_rated_range_km: positions.iter().find_map(|p| p.rated_battery_range_km), // Take the first non None value
        end_rated_range_km: positions.iter().filter_map(|p| p.rated_battery_range_km).last(), // Take the last non None value
        start_position_id: start_position.id,
        end_position_id: end_position.id,
        start_geofence_id: None,
        end_geofence_id: None,
    })
}

pub fn create_charging_from_charges(charges: &[Charges]) -> Option<ChargingProcess> {
    let start_charge = charges.first()?;
    let end_charge = charges.last()?;

    let first_energy_val = charges.iter().find_map(|c| c.charge_energy_added);
    let last_energy_val = if end_charge.charge_energy_added.is_none() || end_charge.charge_energy_added == Some(0.0) {
        let x = charges
            .iter()
            .filter_map(|c| c.charge_energy_added)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        x
    } else {
        end_charge.charge_energy_added
    };
    let charge_energy_added = {
        let e = first_energy_val.zip(last_energy_val).map(|(f, l)| l - f);
        if e < Some(0.0) {
            None
        } else {
            e
        }
    };

    Some(ChargingProcess {
        id: 0,
        start_date: start_charge.date.unwrap_or_default(),
        end_date: end_charge.date,
        charge_energy_added,
        start_ideal_range_km: start_charge.ideal_battery_range_km,
        end_ideal_range_km: end_charge.ideal_battery_range_km,
        start_battery_level: start_charge.battery_level,
        end_battery_level: end_charge.battery_level,
        duration_min: end_charge.date.zip(start_charge.date).map(|(e, s)| e - s).map(|d| (d.num_seconds() as f64 / 60.0).round() as i16),
        outside_temp_avg: Some(charges.iter().filter_map(|c| c.outside_temp).sum::<f32>() / charges.len() as f32),
        start_rated_range_km: start_charge.rated_battery_range_km,
        end_rated_range_km: end_charge.rated_battery_range_km,
        charge_energy_used: calculate_energy_used(charges),
        cost: None,
        car_id: 1,
        position_id: 0,
        address_id: None,
        geofence_id: None,
        charging_status: ChargeStat::Done,
    })
}