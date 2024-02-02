use std::sync::{Arc, Mutex};

use rand::Rng;

use chipmunk::database::{self, tables::token::Token};
use chrono::NaiveDateTime;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tesla_api::{auth::AuthResponse, Vehicles, vehicle_data::VehicleData};

/// Asserts that two floats are approximately equal.
#[macro_export]
macro_rules! approx_eq {
    ($x:expr, $y:expr) => {
        if $x.zip($y).map(|(a, b)| a.abs() - b.abs()) > Some(0.01) {
            panic!(
                "assertion failed: `(left == right)`\n  left: `{:?}`,\n right: `{:?}`",
                $x, $y
            );
        }
    };
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
