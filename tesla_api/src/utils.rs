use chrono::{DateTime, NaiveDateTime};

// Return string of format yyyy-mm-dd hh:mm:ss.sss
// pub fn timestamp_to_string(timestamp: u64) -> Option<String> {
//     let secs = (timestamp / 1000) as i64;
//     let nsecs = (timestamp % 1000 * 1_000_000) as u32;
//     match NaiveDateTime::from_timestamp_opt(secs, nsecs) {
//         Some(naive) => {
//             let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
//             Some(datetime.format("%Y-%m-%d %H:%M:%S.%3f").to_string())
//         }
//         None => None,
//     }
// }

pub fn timestamp_to_naivedatetime(timestamp: Option<u64>) -> Option<NaiveDateTime> {
    match timestamp {
        Some(ts) => {
            let secs = (ts / 1000) as i64;
            let nsecs = (ts % 1000 * 1_000_000) as u32;
            Some(NaiveDateTime::from_timestamp(secs, nsecs))
        }
        None => None,
    }
    // timestamp.and_then(|ts| {
    //     let secs = (ts / 1000) as i64;
    //     let nsecs = (ts % 1000 * 1_000_000) as u32;
    //     NaiveDateTime::from_timestamp(secs, nsecs)
    // })
}

pub fn miles_to_km(miles: &Option<f32>) -> Option<f32> {
    miles.as_ref().map(|value| value * 1.609344)
}

pub fn mph_to_kmh(mph: &Option<i32>) -> Option<f32> {
    mph.as_ref().map(|value| *value as f32 * 1.609344)
}

pub fn get_elevation() -> Option<f32> {
    // TODO: implement this
    None
}
