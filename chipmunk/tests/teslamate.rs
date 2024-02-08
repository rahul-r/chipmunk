use chipmunk::database::{tables::{drive::Drive, position::Position}, Teslamate};

fn create_drive_from_positions(positions: Vec<Position>) -> Drive {
    let start_position = positions.first().unwrap();
    let end_position = positions.last().unwrap();
    Drive {
        id: 0,
        in_progress: false,
        start_date: start_position.date.unwrap_or_default(),
        end_date: end_position.date,
        outside_temp_avg: Some(positions.iter().map(|p| p.outside_temp.unwrap_or(0.0)).sum::<f32>() / positions.len() as f32),
        speed_max: positions.iter().map(|p| p.speed.unwrap() as i32).max().map(|v| v as f32),
        power_max: None,
        power_min: None,
        start_ideal_range_km: start_position.ideal_battery_range_km,
        end_ideal_range_km: end_position.ideal_battery_range_km,
        start_km: start_position.odometer,
        end_km: end_position.odometer,
        distance: end_position.odometer.zip(start_position.odometer).map(|(end, start)| end - start),
        duration_min: end_position.date.zip(start_position.date).map(|(e, s)| e - s).map(|d| d.num_minutes() as i16),
        car_id: start_position.car_id,
        inside_temp_avg: Some(positions.iter().map(|p| p.inside_temp.unwrap_or(0.0)).sum::<f32>() / positions.len() as f32),
        start_address_id: None,
        end_address_id: None,
        start_rated_range_km: start_position.rated_battery_range_km,
        end_rated_range_km: end_position.rated_battery_range_km,
        start_position_id: start_position.id,
        end_position_id: end_position.id,
        start_geofence_id: None,
        end_geofence_id: None,
    }
}

#[tokio::test]
async fn test_teslamate() {
    dotenvy::dotenv().ok();
    let url = std::env::var("TESLAMATE_DATABASE_URL")
        .expect("Cannot get test database URL from environment variable, Please set env `TESLAMATE_DATABASE_URL`");
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let tm_drive = Drive::tm_get_last(&pool).await.unwrap();
    dbg!(&tm_drive);
    let positions = Position::tm_get_for_drive(&pool, tm_drive.car_id, tm_drive.id as i64).await.unwrap();
    dbg!(&positions.len());
    assert_eq!(tm_drive.start_position_id, positions.first().unwrap().id);
    assert_eq!(tm_drive.end_position_id, positions.last().unwrap().id);
    let d = create_drive_from_positions(positions);
    dbg!(&d);
}