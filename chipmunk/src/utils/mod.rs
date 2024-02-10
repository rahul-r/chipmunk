pub mod crypto;
pub mod location;

use std::ops::{Add, Sub};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};

pub fn seconds_remaining(time: NaiveDateTime) -> i32 {
    let duration = DateTime::from_naive_utc_and_offset(time, Utc) - Utc::now();
    duration.num_seconds() as i32
}

pub fn capitalize_string_option(string: Option<String>) -> Option<String> {
    string.map(|s| s.to_ascii_uppercase())
}

pub fn time_diff_minutes_i64(
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
) -> Option<i64> {
    start.zip(end).map(|(st, en)| (en - st).num_minutes())
}

pub fn time_diff_minutes_i16(
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
) -> Option<i16> {
    start
        .zip(end)
        .map(|(st, en)| (en - st).num_minutes() as i16)
}

pub fn time_diff(start: Option<NaiveDateTime>, end: Option<NaiveDateTime>) -> Option<Duration> {
    start.zip(end).map(|(st, en)| en - st)
}

pub fn max_option<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    if let Some(a) = a {
        if let Some(b) = b {
            Some(if a > b { a } else { b })
        } else {
            Some(a)
        }
    } else {
        b
    }
}

pub fn min_option<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    if let Some(a) = a {
        if let Some(b) = b {
            Some(if a < b { a } else { b })
        } else {
            Some(a)
        }
    } else {
        b
    }
}

// pub fn add_option<T: Add<Output = T>>(a: Option<T>, b: Option<T>) -> Option<T> {
//     if let None = a {
//         return b;
//     };

//     if let None = b {
//         return a;
//     };

//     a.zip(b).map(|(a, b)| a + b)
// }

/**
* Returns Some(a - b) or None
*/
pub fn sub_option<T: Sub<Output = T>>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.zip(b).map(|(a, b)| a - b)
}

pub fn avg_option<T: Add<Output = f32>>(a: Option<T>, b: Option<T>) -> Option<f32> {
    a.zip(b).map(|(a, b)| (a + b) / 2.0)
}

// pub fn print_type_of<T>(_: &T) {
//     println!("{}", std::any::type_name::<T>())
// }

#[test]
fn test_max_option() {
    assert_eq!(max_option(Some(2.0), Some(3.0)), Some(3.0));
    assert_eq!(max_option(Some(3.0), Some(2.0)), Some(3.0));
    assert_eq!(max_option(Some(3.2), Some(3.2)), Some(3.2));
    assert_eq!(max_option(Some(13.2), None), Some(13.2));
    assert_eq!(max_option(None, Some(3.42)), Some(3.42));
}

#[test]
fn test_min_option() {
    assert_eq!(min_option(Some(2.0), Some(3.0)), Some(2.0));
    assert_eq!(min_option(Some(3.0), Some(2.0)), Some(2.0));
    assert_eq!(min_option(Some(3.2), Some(3.2)), Some(3.2));
    assert_eq!(min_option(Some(13.2), None), Some(13.2));
    assert_eq!(min_option(None, Some(3.42)), Some(3.42));
}
