/// # SRTM HGT file name format:
/// SRTM data are distributed in two levels:
/// - SRTM1 (for the U.S. and its territories and possessions) with data sampled at one arc-second
///         intervals in latitude and longitude
/// - SRTM3 (for the world) sampled at three arc-seconds.
///
/// Data are divided into one by one degree latitude and longitude tiles in "geographic" projection
///
/// File names refer to the latitude and longitude of the lower left corner of the tile
/// > e.g. N37W105 has its lower left corner at 37 degrees north latitude and 105 degrees west longitude.
///
/// To be more exact, these coordinates refer to the geometric center of the lower left pixel,
/// which in the case of SRTM3 data will be about 90 meters in extent.
///
/// Height files have the extension .HGT and are signed two byte integers.
/// The bytes are in Motorola "big-endian" order with the most significant byte first.
/// Heights are in meters referenced to the WGS84/EGM96 geoid.
/// Data voids are assigned the value -32768.
mod source;

/// Get elevation for a given latitude and longitude.
///
/// # Arguments
/// * `lat`: latitude of the point
/// * `lon`: longitude of the point
///
/// returns: Option<i16>
/// Some(elevation) if the elevation data is available for the given latitude and longitude
///
// TODO: This function loads the *.hgt from the file system on every call. We should pre-load the
// data and use the cached data instead.
pub async fn get_elevation(lat: f64, lon: f64) -> Option<i16> {
    let (name, lat_hgt_base, lon_hgt_base) = hgt_name(lat, lon);
    if !source::file::exists(&name) {
        if let Err(e) = source::esa::fetch(&name).await {
            log::error!("Error fetching elevation: {e}");
            return None;
        }
    }

    source::file::load(&name)
        .and_then(|data| SrtmData::new(lat_hgt_base, lon_hgt_base, data))
        .map_err(|e| log::error!("Error determining elevation: {e}"))
        .ok()
        .and_then(|srtm_data| srtm_data.get_elevation(lat, lon))
}

fn hgt_name(lat: f64, lon: f64) -> (String, i32, i32) {
    let lat_direction = if lat >= 0.0 { "N" } else { "S" };
    let lon_direction = if lon >= 0.0 { "E" } else { "W" };

    let lat_floor = lat.floor();
    let lon_floor = lon.floor();

    let lat_abs = lat_floor.abs();
    let lon_abs = lon_floor.abs();

    let name = format!("{lat_direction}{lat_abs:02}{lon_direction}{lon_abs:03}");
    (name, lat_floor as i32, lon_floor as i32)
}

pub struct SrtmData {
    hgt_data: Vec<u8>,
    latitude: i32,
    longitude: i32,
    points_per_minute: i32,
}

impl SrtmData {
    pub fn new(latitude: i32, longitude: i32, hgt_data: Vec<u8>) -> anyhow::Result<Self> {
        let points_per_minute = SrtmData::get_num_points_per_minute(&hgt_data)?;

        Ok(Self {
            hgt_data,
            latitude,
            longitude,
            points_per_minute,
        })
    }

    pub fn get_elevation(&self, lat: f64, lon: f64) -> Option<i16> {
        let col_in_seconds = ((lon - self.longitude as f64) * 60.0 * 60.0).round() as i32;
        let row_in_seconds = (self.points_per_minute - 1)
            - ((lat - self.latitude as f64) * 60.0 * 60.0).round() as i32;
        let byte_index = (row_in_seconds * self.points_per_minute + col_in_seconds) * 2;

        if byte_index < 0 || byte_index > (self.points_per_minute * self.points_per_minute * 2) {
            return None;
        }

        let byte_pos = byte_index as usize;

        if byte_pos >= self.hgt_data.len() {
            return None;
        }

        let val = i16::from_be_bytes([self.hgt_data[byte_pos], self.hgt_data[byte_pos + 1]]);

        if val == -32768 {
            None
        } else {
            Some(val)
        }
    }

    fn get_num_points_per_minute(data: &[u8]) -> anyhow::Result<i32> {
        let srtm_3_size = 1201 * 1201 * 2; // 1201x1201 samples and each sample is 2 bytes
        let srtm_1_size = 3601 * 3601 * 2; // 3601x3601 samples and each sample is 2 bytes
        if data.len() == srtm_3_size {
            Ok(1201)
        } else if data.len() == srtm_1_size {
            Ok(3601)
        } else {
            anyhow::bail!(
                "Unknown file type. Expected file size {srtm_3_size} or {srtm_1_size}, received {:?}",
                data.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get_elevation() {
        crate::init_log();

        assert_eq!(get_elevation(36.578_444, -118.292_442).await, Some(4409));
        assert_eq!(get_elevation(36.243_148, -116.812_403).await, Some(-84));
        assert_eq!(get_elevation(27.988_990, 86.924_932).await, Some(8741));
        assert_eq!(get_elevation(-37.134_842, 147.005_750).await, Some(666));
        assert_eq!(get_elevation(-13.160_608, -72.538_887).await, Some(1979));
        // assert_eq!(get_elevation(71.386_798, -156.473_866).await, Some(0)); // TODO: Add a source that covers SRTM data for this area
    }
}
