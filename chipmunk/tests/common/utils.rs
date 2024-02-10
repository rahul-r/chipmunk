use std::sync::{Arc, Mutex};

use rand::Rng;

use chipmunk::{database::{self, tables::{charges::Charges, charging_process::ChargingProcess, drive::Drive, position::Position, token::Token}, types::ChargeStat}};
use chrono::NaiveDateTime;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tesla_api::{auth::AuthResponse, Vehicles, vehicle_data::VehicleData};

/// Asserts that two floats are approximately equal.
#[macro_export]
macro_rules! approx_eq {
    ($x:expr, $y:expr) => {{
        if $x.zip($y).map(|(a, b)| a.abs() - b.abs()) > Some(0.01) {
            panic!(
                "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $x, $y
            );
        }
    }};

    ($x:expr, $y:expr, $accuracy:expr) => {{
        if $x.zip($y).map(|(a, b)| a.abs() - b.abs()) > Some($accuracy) {
            panic!(
                "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $x, $y
            );
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

pub fn create_mock_tesla_server(vehicle_data: Arc<Mutex<VehicleData>>, send_response: Arc<Mutex<bool>>) -> mockito::ServerGuard {
    // Request a new server from the pool
    let mut server = mockito::Server::new();

    let vehicle_data_response = move |w: &mut dyn std::io::Write| {
        if *send_response.lock().unwrap() {
            let resp = ApiResponse {
                response: Some(vehicle_data.lock().unwrap().clone()),
            };
            let resp_json = serde_json::to_string(&resp).unwrap();

            w.write_all(resp_json.as_bytes()).unwrap();
        } else {
            w.write_all("{}".as_bytes()).unwrap();
        }
        Ok(())
    };

    server
        .mock("GET", "/vehicles/1/vehicle_data")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_chunked_body(vehicle_data_response)
        .create();

    server
        .mock("GET", "/vehicles")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(create_vehicles_response_json())
        .create();

    let mock_url = server.url();
    std::env::set_var("MOCK_TESLA_BASE_URL", mock_url);
    server
}

pub fn create_mock_osm_server() -> mockito::ServerGuard {
    // Request a new server from the pool
    let mut server = mockito::Server::new();

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
        .create();
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
        token_type: "token_type".to_string(),
    };
    let encryption_key = "secret password acbdefghijklmnop";
    std::env::set_var("TOKEN_ENCRYPTION_KEY", encryption_key);
    Token::db_insert(&pool, tokens, encryption_key)
        .await
        .unwrap();

    pool
}

pub fn ts_no_nanos(ts: NaiveDateTime) -> NaiveDateTime {
    let timestamp = ts.timestamp_millis();
    let secs = timestamp / 1000;
    let nsecs = (timestamp % 1000 * 1_000_000) as u32;
    NaiveDateTime::from_timestamp_opt(secs, nsecs).unwrap()
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
        start_ideal_range_km: positions.iter().filter_map(|p| p.ideal_battery_range_km).next(), // Take the first non None value
        end_ideal_range_km: positions.iter().filter_map(|p| p.ideal_battery_range_km).last(), // Take the last non None value
        start_km: start_position.odometer,
        end_km: end_position.odometer,
        distance: end_position.odometer.zip(start_position.odometer).map(|(end, start)| end - start),
        duration_min: end_position.date.zip(start_position.date).map(|(e, s)| e - s).map(|d| (d.num_seconds() as f64 / 60.0).round() as i16),
        car_id: start_position.car_id,
        inside_temp_avg: Some(filtered_inside_temp.iter().sum::<f32>() / filtered_inside_temp.len() as f32),
        start_address_id: None,
        end_address_id: None,
        start_rated_range_km: positions.iter().filter_map(|p| p.rated_battery_range_km).next(), // Take the first non None value
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
    Some(ChargingProcess {
        id: 0,
        start_date: start_charge.date.unwrap_or_default(),
        end_date: end_charge.date,
        charge_energy_added: end_charge.charge_energy_added,
        start_ideal_range_km: start_charge.ideal_battery_range_km,
        end_ideal_range_km: end_charge.ideal_battery_range_km,
        start_battery_level: start_charge.battery_level,
        end_battery_level: end_charge.battery_level,
        duration_min: end_charge.date.zip(start_charge.date).map(|(e, s)| e - s).map(|d| (d.num_seconds() as f64 / 60.0).round() as i16),
        outside_temp_avg: Some(charges.iter().filter_map(|c| c.outside_temp).sum::<f32>() / charges.len() as f32),
        start_rated_range_km: start_charge.rated_battery_range_km,
        end_rated_range_km: end_charge.rated_battery_range_km,
        charge_energy_used: None,
        cost: None,
        car_id: 1,
        position_id: 0,
        address_id: None,
        geofence_id: None,
        charging_status: ChargeStat::Done,
    })
}