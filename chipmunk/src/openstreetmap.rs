use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Client,
};
use serde::{Deserialize, Serialize};

/// This is an example of how to use the `osm_client` and `reverse_geocode` functions.
/// ```
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let lat = 0.13806939125061035;
///     let lon = 51.51989264641164;
///
///     let client = osm_client()?;
///     let res = reverse_geocode(&client, &lat, &lon).await;
///
///     println!("{:#?}", res);
///
///     Ok(())
/// }
/// ```

// For details, see https://github.com/OpenCageData/address-formatting/blob/master/conf/components.yaml
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct OsmAddress {
    // House number
    pub house_number: Option<String>,
    pub housenumber: Option<String>,
    pub street_number: Option<String>,
    // Road
    pub road: Option<String>,
    pub highway: Option<String>,
    pub footway: Option<String>,
    pub street: Option<String>,
    pub street_name: Option<String>,
    pub path: Option<String>,
    pub pedestrian: Option<String>,
    pub road_reference: Option<String>,
    pub road_reference_intl: Option<String>,
    pub square: Option<String>,
    pub place: Option<String>,
    // Neighbourhood
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city_district: Option<String>,
    pub district: Option<String>,
    pub quarter: Option<String>,
    pub borough: Option<String>,
    pub city_block: Option<String>,
    pub residential: Option<String>,
    pub commercial: Option<String>,
    pub houses: Option<String>,
    pub subdistrict: Option<String>,
    pub subdivision: Option<String>,
    pub ward: Option<String>,
    // Municipality
    pub municipality: Option<String>,
    pub local_administrative_area: Option<String>,
    pub subcounty: Option<String>,
    // Village
    pub village: Option<String>,
    pub hamlet: Option<String>,
    pub locality: Option<String>,
    pub croft: Option<String>,
    // City
    pub city: Option<String>,
    pub town: Option<String>,
    pub township: Option<String>,
    // County
    pub county: Option<String>,
    pub county_code: Option<String>,
    pub department: Option<String>,

    pub state_district: Option<String>,
    pub state: Option<String>,
    // pub ISO3166-2-lvl4: Option<String>,
    pub postcode: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
}

impl OsmAddress {
    /// Format house numbers like "1;2;3" to "1 - 3"
    fn format_house_numbers(numbers: String) -> String {
        let collection = numbers.split(';').collect::<Vec<&str>>();
        if collection.iter().count() > 1 {
            format!(
                "{} - {}",
                collection.first().unwrap_or(&"*"),
                collection.last().unwrap_or(&"*")
            )
        } else {
            numbers
        }
    }

    pub fn get_house_number(&self) -> Option<String> {
        let numbers = if self.house_number.is_some() {
            self.house_number.clone()
        } else if self.housenumber.is_some() {
            self.housenumber.clone()
        } else if self.street_number.is_some() {
            self.street_number.clone()
        } else {
            None
        };

        numbers.map(|n| Self::format_house_numbers(n))
    }

    pub fn get_road(&self) -> Option<String> {
        if self.road.is_some() {
            self.road.clone()
        } else if self.highway.is_some() {
            self.highway.clone()
        } else if self.footway.is_some() {
            self.footway.clone()
        } else if self.street.is_some() {
            self.street.clone()
        } else if self.street_name.is_some() {
            self.street_name.clone()
        } else if self.path.is_some() {
            self.path.clone()
        } else if self.pedestrian.is_some() {
            self.pedestrian.clone()
        } else if self.road_reference.is_some() {
            self.road_reference.clone()
        } else if self.road_reference_intl.is_some() {
            self.road_reference_intl.clone()
        } else if self.square.is_some() {
            self.square.clone()
        } else if self.place.is_some() {
            self.place.clone()
        } else {
            None
        }
    }

    pub fn get_neighbourhood(&self) -> Option<String> {
        if self.neighbourhood.is_some() {
            self.neighbourhood.clone()
        } else if self.suburb.is_some() {
            self.suburb.clone()
        } else if self.city_district.is_some() {
            self.city_district.clone()
        } else if self.district.is_some() {
            self.district.clone()
        } else if self.quarter.is_some() {
            self.quarter.clone()
        } else if self.borough.is_some() {
            self.borough.clone()
        } else if self.city_block.is_some() {
            self.city_block.clone()
        } else if self.residential.is_some() {
            self.residential.clone()
        } else if self.commercial.is_some() {
            self.commercial.clone()
        } else if self.houses.is_some() {
            self.houses.clone()
        } else if self.subdistrict.is_some() {
            self.subdistrict.clone()
        } else if self.subdivision.is_some() {
            self.subdivision.clone()
        } else if self.ward.is_some() {
            self.ward.clone()
        } else {
            None
        }
    }

    pub fn get_city(&self) -> Option<String> {
        if self.city.is_some() {
            self.city.clone()
        } else if self.town.is_some() {
            self.town.clone()
        } else if self.township.is_some() {
            self.township.clone()
        } else {
            None
        }
    }

    pub fn get_county(&self) -> Option<String> {
        if self.county.is_some() {
            self.county.clone()
        } else if self.county_code.is_some() {
            self.county_code.clone()
        } else if self.department.is_some() {
            self.department.clone()
        } else {
            None
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Extratags {
    pub distance: Option<String>,
    pub wikitags: Option<String>,
    pub url: Option<String>,
    pub layer: Option<String>,
    pub function: Option<String>,
    pub operator: Option<String>,
    pub wikidata: Option<String>,
    pub architect: Option<String>,
    pub wikipedia: Option<String>,
    pub start_date: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct NameDetails {
    pub r#ref: Option<String>,
    pub name: Option<String>,
    pub alt_name: Option<String>,
    pub old_name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct OsmResponse {
    pub place_id: Option<i32>,
    pub licence: Option<String>,
    pub osm_type: Option<String>,
    pub osm_id: Option<i64>,
    pub lat: Option<String>,
    pub lon: Option<String>,
    pub place_rank: Option<i32>,
    pub category: Option<String>,
    pub r#type: Option<String>,
    pub importance: Option<f32>,
    pub addresstype: Option<String>,
    pub display_name: Option<String>,
    pub name: Option<String>,
    pub address: Option<OsmAddress>,
    pub extratags: Option<Extratags>,
    pub namedetails: Option<NameDetails>,
    pub boundingbox: Option<[String; 4]>,
    error: Option<String>,
}

impl OsmResponse {
    pub fn get_name(&self) -> Option<String> {
        /// Cleanup the name string
        /// Returns trimmed Some(string) if the string is not empty else return None
        fn cleanup(name: &Option<String>) -> Option<String> {
            name.as_ref().map(|s| s.trim()).and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.into())
                }
            })
        }

        cleanup(&self.name) // Return name if it is valid
            .or_else(|| {
                self.namedetails
                    .as_ref()
                    .and_then(|details| cleanup(&details.name)) // else return `namedetails.name` if it is valid
            })
            .or_else(|| {
                self.namedetails
                    .as_ref()
                    .and_then(|details| cleanup(&details.alt_name)) // else return `namedetails.alt_name` if it is valid
            }) // else return None
    }

    pub fn get_house_number(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.get_house_number())
    }
    pub fn get_raw_house_number(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.house_number.clone())
    }
    pub fn get_road(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.get_road())
    }
    pub fn get_city(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.get_city())
    }
    pub fn get_postcode(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.postcode.clone())
    }
    pub fn get_neighbourhood(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.get_neighbourhood())
    }
    pub fn get_county(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.get_county())
    }
    pub fn get_state_district(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.state_district.clone())
    }
    pub fn get_state(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.state.clone())
    }
    pub fn get_country(&self) -> Option<String> {
        self.address.as_ref().and_then(|a| a.country.clone())
    }
    pub fn get_formatted_display_name(&self) -> Option<String> {
        if let Some(formatted_house_number) = self.get_house_number() {
            let raw_house_number = self.get_raw_house_number().unwrap_or("".to_string());
            self.display_name
                .as_ref()
                .map(|s| s.replace(&raw_house_number, &formatted_house_number))
        } else {
            self.display_name.clone()
        }
    }
}

const BASE_URL: &str = "https://nominatim.openstreetmap.org";

pub fn osm_client() -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Tesla-Rust"));

    let client = Client::builder().default_headers(headers).build()?;

    Ok(client)
}

pub async fn reverse_geocode(client: &Client, lat: &f32, lon: &f32) -> anyhow::Result<OsmResponse> {
    let res = client
        .get(format!("{BASE_URL}/reverse"))
        .query(&[
            ("lat", lat.to_string()),
            ("lon", lon.to_string()),
            ("addressdetails", "1".into()),
            ("extratags", "1".into()),
            ("namedetails", "1".into()),
            ("zoom", "19".into()),
            ("format", "jsonv2".into()),
        ])
        .send()
        .await?;

    if res.status() != reqwest::StatusCode::OK {
        anyhow::bail!("Unexpected response code: {}", res.status());
    }

    let osm_response = res.json::<OsmResponse>().await?;

    if let Some(error) = osm_response.error {
        anyhow::bail!(error);
    }

    Ok(osm_response)
}

#[allow(dead_code, unused)]
pub async fn geocode(address: String) -> Option<OsmResponse> {
    // TODO: use the '/lookup' endpoint of openstreetmap endpoint and return the results
    // See the 'details' function in lib/teslamate/locations/geocoder.ex for more info
    todo!()
}

#[tokio::test]
async fn test_reverse_geocode() {
    let lat = 64.7529099405634;
    let lon = -147.35390714170856;

    let client = osm_client().unwrap();
    let res = reverse_geocode(&client, &lat, &lon).await.unwrap();

    assert_eq!(res.display_name, Some("United States Post Office, 2nd Avenue, Highland Park, North Pole, Fairbanks North Star, Alaska, 99705, United States".into()));
    // assert_eq!(res.lat, Some("64.75273".into()));
    // assert_eq!(res.lon, Some("-147.35391".into()));
    assert_eq!(res.name, Some("United States Post Office".into()));
    assert_eq!(res.get_house_number(), None);
    assert_eq!(res.get_road(), Some("2nd Avenue".into()));
    assert_eq!(res.get_neighbourhood(), None);
    assert_eq!(res.get_city(), Some("North Pole".into()));
    assert_eq!(res.get_county(), Some("Fairbanks North Star".into()));
    assert_eq!(res.get_postcode(), Some("99705".into()));
    assert_eq!(res.get_state(), Some("Alaska".into()));
    assert_eq!(res.get_state_district(), None);
    assert_eq!(res.get_country(), Some("United States".into()));
}
