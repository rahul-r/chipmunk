use chipmunk::database::{tables::{drive::Drive, position::Position}, Teslamate};
use chrono::Duration;

use crate::common::utils::create_drive_from_positions;

pub mod common;

fn validate_drive(drive: &Drive, expected: &Drive) {
    assert_eq!(drive.in_progress, expected.in_progress);
    assert!(drive.start_date - expected.start_date < Duration::seconds(1));
    assert_eq!(drive.end_date.zip(expected.end_date).map(|(de, ee)| de - ee < Duration::seconds(1)), Some(true));
    approx_eq!(drive.outside_temp_avg, expected.outside_temp_avg, 0.1);
    assert_eq!(drive.speed_max, expected.speed_max);
    assert_eq!(drive.power_max, expected.power_max);
    assert_eq!(drive.power_min, expected.power_min);
    assert_eq!(drive.start_ideal_range_km, expected.start_ideal_range_km);
    assert_eq!(drive.end_ideal_range_km, expected.end_ideal_range_km);
    assert_eq!(drive.start_km, expected.start_km);
    assert_eq!(drive.end_km, expected.end_km);
    approx_eq!(drive.distance, expected.distance);
    assert_eq!(drive.duration_min, expected.duration_min);
    assert_eq!(drive.car_id, expected.car_id);
    approx_eq!(drive.inside_temp_avg, expected.inside_temp_avg, 0.1);
    assert_eq!(drive.start_rated_range_km, expected.start_rated_range_km);
    assert_eq!(drive.end_rated_range_km, expected.end_rated_range_km);
    assert_eq!(drive.start_position_id, expected.start_position_id);
    assert_eq!(drive.end_position_id, expected.end_position_id);
    // IGNORE THIS assert_eq!(drive.id, expected.id);
    // IGNORE THIS assert_eq!(drive.start_address_id, expected.start_address_id);
    // IGNORE THIS assert_eq!(drive.end_address_id, expected.end_address_id);
    // IGNORE THIS assert_eq!(drive.start_geofence_id, expected.start_geofence_id);
    // IGNORE THIS assert_eq!(drive.end_geofence_id, expected.end_geofence_id);
}

#[tokio::test]
async fn test_teslamate() {
    dotenvy::dotenv().ok();
    let url = std::env::var("TESLAMATE_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TESLAMATE_DATABASE_URL`");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();

    // let tm_drive = Drive::tm_get_id(&pool, 1196).await.unwrap();
    // let positions = Position::tm_get_for_drive(&pool, tm_drive.car_id, tm_drive.id as i64).await.unwrap();
    // let d = create_drive_from_positions(&positions);
    // validate_drive(&tm_drive, &d.unwrap());
    
    // Test the last 1000 drives
    let last_drive_id = Drive::tm_get_last(&pool).await.unwrap().id as i64;
    let first_drive_id_to_test = match last_drive_id - 1000 {
        id if id > 0 => id,
        _ => 0
    };
    for id in first_drive_id_to_test..=last_drive_id {
        println!("::> Testing drive id: {}", id);
        let tm_drive = match Drive::tm_get_id(&pool, id).await {
            Ok(d) => d,
            Err(e) => {
                eprintln!("::> Drive ID {id}: {e}");
                continue;
            }
        };
        if tm_drive.end_date.is_none() {
            eprintln!("::> Drive ID {id} has no end date");
            continue;
        }
        let positions = Position::tm_get_for_drive(&pool, tm_drive.car_id, tm_drive.id as i64).await.unwrap();
        let d = create_drive_from_positions(&positions);
        assert!(d.is_some());
        validate_drive(&tm_drive, &d.unwrap());
    }
}