use std::{fs::File, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ShiftState {
    P,
    R,
    N,
    D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChargingState {
    // Tesla API options are 'null', 'Complete', 'Charging', 'NoPower', 'Stopped', 'Starting', and 'Disconnected'
    Complete,
    Charging,
    NoPower,
    Stopped,
    Starting,
    Disconnected,
    // Add a field to track if a new charging state is added to the tesla API
    Unknown(String),
}

fn from_charging_state_str<'de, T>(deserializer: T) -> Result<Option<ChargingState>, T::Error>
where
    T: Deserializer<'de>,
{
    let s: &str = match Deserialize::deserialize(deserializer) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Error deserializing charging state: {}", e);
            return Ok(None);
        }
    };
    let state = match s {
        "Complete" => ChargingState::Complete,
        "Charging" => ChargingState::Charging,
        "NoPower" => ChargingState::NoPower,
        "Stopped" => ChargingState::Stopped,
        "Starting" => ChargingState::Starting,
        "Disconnected" => ChargingState::Disconnected,
        unknown => {
            log::warn!(
                "Unknown charging state `{}`. Consider updating `ChargingState` enum",
                unknown
            );
            ChargingState::Unknown(unknown.to_string())
        }
    };
    Ok(Some(state))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChargeState {
    pub battery_heater_on: Option<bool>,
    pub battery_level: Option<i16>,
    pub battery_range: Option<f32>,
    pub charge_amps: Option<i32>,
    pub charge_current_request: Option<i32>,
    pub charge_current_request_max: Option<i32>,
    pub charge_enable_request: Option<bool>,
    pub charge_energy_added: Option<f32>,
    pub charge_limit_soc: Option<i32>,
    pub charge_limit_soc_max: Option<i32>,
    pub charge_limit_soc_min: Option<i32>,
    pub charge_limit_soc_std: Option<i32>,
    pub charge_miles_added_ideal: Option<f32>,
    pub charge_miles_added_rated: Option<f32>,
    pub charge_port_cold_weather_mode: Option<bool>,
    pub charge_port_color: Option<String>,
    pub charge_port_door_open: Option<bool>,
    pub charge_port_latch: Option<String>,
    pub charge_rate: Option<f32>,
    pub charger_actual_current: Option<i16>,
    // pub charger_phases: Option<String>,
    pub charger_phases: Option<i16>,
    pub charger_pilot_current: Option<i16>,
    pub charger_power: Option<i16>,
    pub charger_voltage: Option<i16>,
    #[serde(deserialize_with = "from_charging_state_str")]
    pub charging_state: Option<ChargingState>,
    pub conn_charge_cable: Option<String>,
    pub est_battery_range: Option<f32>,
    pub fast_charger_brand: Option<String>,
    pub fast_charger_present: Option<bool>,
    pub fast_charger_type: Option<String>,
    pub ideal_battery_range: Option<f32>,
    pub managed_charging_active: Option<bool>,
    pub managed_charging_start_time: Option<i32>,
    pub managed_charging_user_canceled: Option<bool>,
    pub max_range_charge_counter: Option<i32>,
    pub minutes_to_full_charge: Option<i32>,
    pub not_enough_power_to_heat: Option<bool>,
    pub off_peak_charging_enabled: Option<bool>,
    pub off_peak_charging_times: Option<String>,
    pub off_peak_hours_end_time: Option<i32>,
    pub preconditioning_enabled: Option<bool>,
    pub preconditioning_times: Option<String>,
    pub scheduled_charging_mode: Option<String>,
    pub scheduled_charging_pending: Option<bool>,
    pub scheduled_charging_start_time: Option<i32>,
    pub scheduled_charging_start_time_app: Option<i32>,
    pub scheduled_departure_time: Option<i32>,
    pub supercharger_session_trip_planner: Option<bool>,
    pub time_to_full_charge: Option<f32>,
    pub timestamp: Option<u64>,
    pub trip_charging: Option<bool>,
    pub usable_battery_level: Option<i16>,
    pub user_charge_enable_request: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClimateState {
    pub allow_cabin_overheat_protection: Option<bool>,
    pub auto_seat_climate_left: Option<bool>,
    pub auto_seat_climate_right: Option<bool>,
    pub battery_heater: Option<bool>,
    pub battery_heater_no_power: Option<bool>,
    pub cabin_overheat_protection: Option<String>,
    pub cabin_overheat_protection_actively_cooling: Option<bool>,
    pub climate_keeper_mode: Option<String>,
    pub cop_activation_temperature: Option<String>,
    pub defrost_mode: Option<i32>,
    pub driver_temp_setting: Option<f32>,
    pub fan_status: Option<i32>,
    pub hvac_auto_request: Option<String>,
    pub inside_temp: Option<f32>,
    pub is_auto_conditioning_on: Option<bool>,
    pub is_climate_on: Option<bool>,
    pub is_front_defroster_on: Option<bool>,
    pub is_preconditioning: Option<bool>,
    pub is_rear_defroster_on: Option<bool>,
    pub left_temp_direction: Option<i32>,
    pub max_avail_temp: Option<f32>,
    pub min_avail_temp: Option<f32>,
    pub outside_temp: Option<f32>,
    pub passenger_temp_setting: Option<f32>,
    pub remote_heater_control_enabled: Option<bool>,
    pub right_temp_direction: Option<i32>,
    pub seat_heater_left: Option<i32>,
    pub seat_heater_rear_center: Option<i32>,
    pub seat_heater_rear_left: Option<i32>,
    pub seat_heater_rear_right: Option<i32>,
    pub seat_heater_right: Option<i32>,
    pub side_mirror_heaters: Option<bool>,
    pub supports_fan_only_cabin_overheat_protection: Option<bool>,
    pub timestamp: Option<u64>,
    pub wiper_blade_heater: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DriveState {
    pub active_route_destination: Option<String>,
    pub active_route_energy_at_arrival: Option<i32>,
    pub active_route_latitude: Option<f32>,
    pub active_route_longitude: Option<f32>,
    pub active_route_miles_to_arrival: Option<f32>,
    pub active_route_minutes_to_arrival: Option<f32>,
    pub active_route_traffic_minutes_delay: Option<f32>,
    pub gps_as_of: Option<i32>,
    pub heading: Option<i32>,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    pub native_latitude: Option<f32>,
    pub native_location_supported: Option<i32>,
    pub native_longitude: Option<f32>,
    pub native_type: Option<String>,
    pub power: Option<f32>,
    pub shift_state: Option<ShiftState>,
    pub speed: Option<i32>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct GuiSettings {
    pub gui_24_hour_time: Option<bool>,
    pub gui_charge_rate_units: Option<String>,
    pub gui_distance_units: Option<String>,
    pub gui_range_display: Option<String>,
    pub gui_temperature_units: Option<String>,
    pub gui_tirepressure_units: Option<String>,
    pub show_range_units: Option<bool>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VehicleConfig {
    pub aux_park_lamps: Option<String>,
    pub badge_version: Option<i32>,
    pub can_accept_navigation_requests: Option<bool>,
    pub can_actuate_trunks: Option<bool>,
    pub car_special_type: Option<String>,
    pub car_type: Option<String>,
    pub charge_port_type: Option<String>,
    pub cop_user_set_temp_supported: Option<bool>,
    pub dashcam_clip_save_supported: Option<bool>,
    pub default_charge_to_max: Option<bool>,
    pub driver_assist: Option<String>,
    pub ece_restrictions: Option<bool>,
    pub efficiency_package: Option<String>,
    pub eu_vehicle: Option<bool>,
    pub exterior_color: Option<String>,
    pub exterior_trim: Option<String>,
    pub exterior_trim_override: Option<String>,
    pub has_air_suspension: Option<bool>,
    pub has_ludicrous_mode: Option<bool>,
    pub has_seat_cooling: Option<bool>,
    pub headlamp_type: Option<String>,
    pub interior_trim_type: Option<String>,
    pub key_version: Option<i32>,
    pub motorized_charge_port: Option<bool>,
    pub paint_color_override: Option<String>,
    pub performance_package: Option<String>,
    pub plg: Option<bool>,
    pub pws: Option<bool>,
    pub rear_drive_unit: Option<String>,
    pub rear_seat_heaters: Option<i32>,
    pub rear_seat_type: Option<i32>,
    pub rhd: Option<bool>,
    pub roof_color: Option<String>,
    pub seat_type: Option<String>,
    pub spoiler_type: Option<String>,
    pub sun_roof_installed: Option<bool>,
    pub supports_qr_pairing: Option<bool>,
    pub third_row_seats: Option<String>,
    pub timestamp: Option<u64>,
    pub trim_badging: Option<String>,
    pub use_range_badging: Option<bool>,
    pub utc_offset: Option<i32>,
    pub webcam_selfie_supported: Option<bool>,
    pub webcam_supported: Option<bool>,
    pub wheel_type: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub a2dp_source_name: Option<String>,
    pub audio_volume: Option<f32>,
    pub audio_volume_increment: Option<f32>,
    pub audio_volume_max: Option<f32>,
    pub media_playback_status: Option<String>,
    pub now_playing_album: Option<String>,
    pub now_playing_artist: Option<String>,
    pub now_playing_duration: Option<i32>,
    pub now_playing_elapsed: Option<i32>,
    pub now_playing_source: Option<String>,
    pub now_playing_station: Option<String>,
    pub now_playing_title: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MediaState {
    pub remote_control_enabled: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SoftwareUpdate {
    pub download_perc: Option<i32>,
    pub expected_duration_sec: Option<i32>,
    pub install_perc: Option<i32>,
    pub status: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpeedLimitMode {
    pub active: Option<bool>,
    pub current_limit_mph: Option<f32>,
    pub max_limit_mph: Option<f32>,
    pub min_limit_mph: Option<f32>,
    pub pin_code_set: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub api_version: Option<i32>,
    pub autopark_state_v3: Option<String>,
    pub autopark_style: Option<String>,
    pub calendar_supported: Option<bool>,
    pub car_version: Option<String>,
    pub center_display_state: Option<i32>,
    pub dashcam_clip_save_available: Option<bool>,
    pub dashcam_state: Option<String>,
    pub df: Option<i32>,
    pub dr: Option<i32>,
    pub fd_window: Option<i32>,
    pub feature_bitmask: Option<String>,
    pub fp_window: Option<i32>,
    pub ft: Option<i32>,
    pub homelink_device_count: Option<i32>,
    pub homelink_nearby: Option<bool>,
    pub is_user_present: Option<bool>,
    pub last_autopark_error: Option<String>,
    pub locked: Option<bool>,
    pub media_info: Option<MediaInfo>,
    pub media_state: Option<MediaState>,
    pub notifications_supported: Option<bool>,
    pub odometer: Option<f32>,
    pub parsed_calendar_supported: Option<bool>,
    pub pf: Option<i32>,
    pub pr: Option<i32>,
    pub rd_window: Option<i32>,
    pub remote_start: Option<bool>,
    pub remote_start_enabled: Option<bool>,
    pub remote_start_supported: Option<bool>,
    pub rp_window: Option<i32>,
    pub rt: Option<i32>,
    pub santa_mode: Option<i32>,
    pub sentry_mode: Option<bool>,
    pub sentry_mode_available: Option<bool>,
    pub service_mode: Option<bool>,
    pub service_mode_plus: Option<bool>,
    pub smart_summon_available: Option<bool>,
    pub software_update: Option<SoftwareUpdate>,
    pub speed_limit_mode: Option<SpeedLimitMode>,
    pub summon_standby_mode_enabled: Option<bool>,
    pub timestamp: Option<u64>,
    pub tpms_hard_warning_fl: Option<bool>,
    pub tpms_hard_warning_fr: Option<bool>,
    pub tpms_hard_warning_rl: Option<bool>,
    pub tpms_hard_warning_rr: Option<bool>,
    pub tpms_last_seen_pressure_time_fl: Option<i32>,
    pub tpms_last_seen_pressure_time_fr: Option<i32>,
    pub tpms_last_seen_pressure_time_rl: Option<i32>,
    pub tpms_last_seen_pressure_time_rr: Option<i32>,
    pub tpms_pressure_fl: Option<f32>,
    pub tpms_pressure_fr: Option<f32>,
    pub tpms_pressure_rl: Option<f32>,
    pub tpms_pressure_rr: Option<f32>,
    pub tpms_rcp_front_value: Option<f32>,
    pub tpms_rcp_rear_value: Option<f32>,
    pub tpms_soft_warning_fl: Option<bool>,
    pub tpms_soft_warning_fr: Option<bool>,
    pub tpms_soft_warning_rl: Option<bool>,
    pub tpms_soft_warning_rr: Option<bool>,
    pub valet_mode: Option<bool>,
    pub valet_pin_needed: Option<bool>,
    pub vehicle_name: Option<String>,
    pub vehicle_self_test_progress: Option<i32>,
    pub vehicle_self_test_requested: Option<bool>,
    pub webcam_available: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VehicleData {
    pub id: Option<u64>,
    pub user_id: Option<i32>,
    pub vehicle_id: Option<u64>,
    pub vin: Option<String>,
    pub display_name: Option<String>,
    pub option_codes: Option<String>,
    pub color: Option<String>,
    pub access_type: Option<String>,
    pub tokens: Vec<String>,
    pub state: Option<String>,
    pub in_service: Option<bool>,
    pub id_s: Option<String>,
    pub calendar_enabled: Option<bool>,
    pub api_version: Option<i32>,
    pub backseat_token: Option<String>,
    pub backseat_token_updated_at: Option<i32>,
    pub ble_autopair_enrolled: Option<bool>,
    pub charge_state: Option<ChargeState>,
    pub climate_state: Option<ClimateState>,
    pub drive_state: Option<DriveState>,
    pub gui_settings: Option<GuiSettings>,
    pub vehicle_config: Option<VehicleConfig>,
    pub vehicle_state: Option<VehicleState>,
}

impl Default for VehicleData {
    fn default() -> Self {
        Self {
            id: Default::default(),
            user_id: Default::default(),
            vehicle_id: Default::default(),
            vin: Default::default(),
            display_name: Default::default(),
            option_codes: Default::default(),
            color: Default::default(),
            access_type: Default::default(),
            tokens: Default::default(),
            state: Default::default(),
            in_service: Default::default(),
            id_s: Default::default(),
            calendar_enabled: Default::default(),
            api_version: Default::default(),
            backseat_token: Default::default(),
            backseat_token_updated_at: Default::default(),
            ble_autopair_enrolled: Default::default(),
            charge_state: Some(ChargeState::default()),
            climate_state: Some(ClimateState::default()),
            drive_state: Some(DriveState::default()),
            gui_settings: Some(GuiSettings::default()),
            vehicle_config: Some(VehicleConfig::default()),
            vehicle_state: Some(VehicleState::default()),
        }
    }
}

impl VehicleData {
    pub fn from_response_json(json: &str) -> anyhow::Result<Self> {
        serde_json::from_str::<VehicleData>(json).map_err(|e| e.into())
    }

    #[allow(unused)]
    pub fn load_json_file(filename: &Path) -> anyhow::Result<Self> {
        let reader = std::io::BufReader::new(File::open(filename)?);
        let data: VehicleData = serde_json::from_reader(reader)?;
        Ok(data)
    }

    #[allow(unused)]
    pub fn write_json_file(&self, filename: &Path) -> anyhow::Result<()> {
        serde_json::to_writer(&File::create(filename)?, self)?;
        Ok(())
    }

    pub fn timestamp_epoch(&self) -> Option<u64> {
        self.vehicle_state
            .as_ref()
            .and_then(|vehicle_state| vehicle_state.timestamp)
    }

    pub fn timestamp_utc(&self) -> Option<DateTime<Utc>> {
        let timestamp = self.vehicle_state.as_ref()?.timestamp?;
        let secs = (timestamp / 1000) as i64;
        let nsecs = (timestamp % 1000 * 1_000_000) as u32;
        DateTime::from_timestamp(secs, nsecs)
    }

    pub fn location(&self) -> Option<(f32, f32)> {
        let drive_state = self.drive_state.as_ref()?;
        let lat = drive_state.latitude?;
        let lon = drive_state.longitude?;
        Some((lat, lon))
    }

    pub fn is_driving(&self) -> bool {
        self.drive_state
            .as_ref()
            .and_then(|d| d.shift_state.as_ref())
            .map(|s| *s == ShiftState::D)
            .unwrap_or(false)
    }
}
