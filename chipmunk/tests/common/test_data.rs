#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::too_many_lines)]

use chipmunk::database::tables::state::StateStatus;
use chrono::{DateTime, Utc};
use rand::{self, Rng};

use tesla_api::vehicle_data::{
    ChargeState, ChargingState, ClimateState, DriveState, GuiSettings, ShiftState, VehicleConfig,
    VehicleData, VehicleState,
};

pub fn get_charge_state(timestamp: u64) -> ChargeState {
    ChargeState {
        battery_heater_on: Some(false),
        battery_level: Some(50),
        battery_range: Some(150.0),
        charge_amps: Some(30),
        charge_current_request: Some(40),
        charge_current_request_max: Some(40),
        charge_enable_request: Some(true),
        charge_energy_added: Some(36.0),
        charge_limit_soc: Some(80),
        charge_limit_soc_max: Some(100),
        charge_limit_soc_min: Some(50),
        charge_limit_soc_std: Some(90),
        charge_miles_added_ideal: Some(150.0),
        charge_miles_added_rated: Some(150.0),
        charge_port_cold_weather_mode: Some(false),
        charge_port_color: Some("<invalid>".into()),
        charge_port_door_open: Some(false),
        charge_port_latch: Some("Engaged".into()),
        charge_rate: Some(0.0),
        charger_actual_current: Some(0),
        charger_phases: None,
        charger_pilot_current: Some(40),
        charger_power: Some(0),
        charger_voltage: Some(rand::thread_rng().gen_range(10..400)),
        charging_state: Some(ChargingState::Disconnected),
        conn_charge_cable: Some("<invalid>".into()),
        est_battery_range: Some(rand::thread_rng().gen_range(0.0..400.0)),
        fast_charger_brand: Some("<invalid>".into()),
        fast_charger_present: Some(false),
        fast_charger_type: Some("<invalid>".into()),
        ideal_battery_range: Some(rand::thread_rng().gen_range(10.0..400.0)),
        managed_charging_active: Some(false),
        managed_charging_start_time: None,
        managed_charging_user_canceled: Some(false),
        max_range_charge_counter: Some(0),
        minutes_to_full_charge: Some(0),
        not_enough_power_to_heat: None,
        off_peak_charging_enabled: Some(false),
        off_peak_charging_times: None,
        off_peak_hours_end_time: None,
        preconditioning_enabled: Some(false),
        preconditioning_times: None,
        scheduled_charging_mode: Some("Off".into()),
        scheduled_charging_pending: Some(false),
        scheduled_charging_start_time: None,
        scheduled_charging_start_time_app: Some(0),
        scheduled_departure_time: None,
        supercharger_session_trip_planner: Some(false),
        time_to_full_charge: Some(0.0),
        timestamp: Some(timestamp),
        trip_charging: Some(false),
        usable_battery_level: Some(rand::thread_rng().gen_range(2..90)),
        user_charge_enable_request: None,
    }
}

pub fn get_climate_state(timestamp: u64) -> ClimateState {
    ClimateState {
        timestamp: Some(timestamp),
        fan_status: Some(rand::thread_rng().gen_range(0..10)),
        inside_temp: Some(rand::thread_rng().gen_range(-20.0..120.0)),
        defrost_mode: Some(rand::thread_rng().gen_range(0..1)),
        outside_temp: Some(rand::thread_rng().gen_range(-50.0..120.0)),
        is_climate_on: Some(false),
        battery_heater: Some(false),
        max_avail_temp: Some(30.0),
        min_avail_temp: Some(15.0),
        seat_heater_left: Some(rand::thread_rng().gen_range(0..3)),
        hvac_auto_request: Some("On".into()),
        seat_heater_right: Some(rand::thread_rng().gen_range(0..3)),
        is_preconditioning: Some(false),
        wiper_blade_heater: Some(false),
        climate_keeper_mode: Some("off".into()),
        driver_temp_setting: Some(rand::thread_rng().gen_range(15.0..30.0)),
        left_temp_direction: Some(rand::thread_rng().gen_range(0..360)),
        side_mirror_heaters: Some(false),
        is_rear_defroster_on: Some(false),
        right_temp_direction: Some(rand::thread_rng().gen_range(0..360)),
        is_front_defroster_on: Some(false),
        seat_heater_rear_left: Some(rand::thread_rng().gen_range(0..3)),
        auto_seat_climate_left: Some(false),
        passenger_temp_setting: Some(rand::thread_rng().gen_range(15.0..30.0)),
        seat_heater_rear_right: Some(rand::thread_rng().gen_range(0..3)),
        auto_seat_climate_right: Some(false),
        battery_heater_no_power: None,
        is_auto_conditioning_on: Some(false),
        seat_heater_rear_center: Some(rand::thread_rng().gen_range(0..3)),
        cabin_overheat_protection: Some("On".into()),
        cop_activation_temperature: Some("High".into()),
        remote_heater_control_enabled: Some(false),
        allow_cabin_overheat_protection: Some(true),
        cabin_overheat_protection_actively_cooling: Some(false),
        supports_fan_only_cabin_overheat_protection: Some(true),
    }
}

pub fn get_drive_state(timestamp: u64) -> DriveState {
    DriveState {
        active_route_destination: Some("Palo Alto".into()),
        active_route_energy_at_arrival: Some(10),
        active_route_latitude: Some(0.0),
        active_route_longitude: Some(0.0),
        active_route_miles_to_arrival: Some(100.0),
        active_route_minutes_to_arrival: Some(90.0),
        active_route_traffic_minutes_delay: Some(0.0),
        gps_as_of: Some(0),
        heading: Some(0),
        latitude: Some(0.0),
        longitude: Some(0.0),
        native_latitude: Some(0.0),
        native_location_supported: Some(1),
        native_longitude: Some(0.0),
        native_type: Some("wgs".into()),
        power: Some(rand::thread_rng().gen_range(0.0..100.0)),
        shift_state: Some(ShiftState::D),
        speed: Some(rand::thread_rng().gen_range(0..120)),
        timestamp: Some(timestamp),
    }
}
pub fn get_gui_settings(timestamp: u64) -> GuiSettings {
    GuiSettings {
        gui_24_hour_time: Some(false),
        gui_charge_rate_units: Some("kW".into()),
        gui_distance_units: Some("mi/hr".into()),
        gui_range_display: Some("Rated".into()),
        gui_temperature_units: Some("F".into()),
        gui_tirepressure_units: Some("Psi".into()),
        show_range_units: Some(false),
        timestamp: Some(timestamp),
    }
}
pub fn get_vehicle_config(timestamp: u64) -> VehicleConfig {
    VehicleConfig {
        plg: Some(false),
        pws: Some(false),
        rhd: Some(false),
        car_type: Some("model3".into()),
        seat_type: None,
        timestamp: Some(timestamp),
        eu_vehicle: Some(false),
        roof_color: Some("RoofColorGlass".into()),
        utc_offset: Some(-28800),
        wheel_type: Some("Pinwheel20".into()),
        key_version: Some(2),
        spoiler_type: Some("None".into()),
        trim_badging: Some("74d".into()),
        badge_version: Some(0),
        driver_assist: Some("TeslaAP3".into()),
        exterior_trim: Some("Chrome".into()),
        headlamp_type: Some("Premium".into()),
        aux_park_lamps: Some("NaPremium".into()),
        exterior_color: Some("Cherry".into()),
        rear_seat_type: Some(0),
        rear_drive_unit: Some("HGDTI875TGF".into()),
        third_row_seats: Some("None".into()),
        car_special_type: Some("base".into()),
        charge_port_type: Some("US".into()),
        ece_restrictions: Some(false),
        has_seat_cooling: Some(false),
        webcam_supported: Some(true),
        rear_seat_heaters: Some(1),
        use_range_badging: Some(true),
        can_actuate_trunks: Some(true),
        efficiency_package: Some("Default".into()),
        has_air_suspension: Some(false),
        has_ludicrous_mode: Some(false),
        interior_trim_type: Some("Green".into()),
        sun_roof_installed: None,
        performance_package: Some("Plaid".into()),
        supports_qr_pairing: Some(false),
        paint_color_override: Some("".into()),
        default_charge_to_max: Some(false),
        motorized_charge_port: Some(true),
        exterior_trim_override: Some("".into()),
        webcam_selfie_supported: Some(true),
        cop_user_set_temp_supported: Some(true),
        dashcam_clip_save_supported: Some(true),
        can_accept_navigation_requests: Some(true),
    }
}

pub fn get_vehicle_state(timestamp: u64) -> VehicleState {
    VehicleState {
        api_version: Some(70),
        autopark_state_v3: Some("standby".into()),
        autopark_style: Some("dead_man".into()),
        calendar_supported: Some(true),
        car_version: Some("2023.11.22.3.4 a8f7df98b76c".into()),
        center_display_state: Some(0),
        dashcam_clip_save_available: Some(true),
        dashcam_state: Some("Recording".into()),
        df: Some(0),
        dr: Some(0),
        fd_window: Some(0),
        feature_bitmask: Some("ffffffff,ffff".into()),
        fp_window: Some(0),
        ft: Some(0),
        homelink_device_count: Some(0),
        homelink_nearby: Some(false),
        is_user_present: Some(false),
        last_autopark_error: Some("no_error".into()),
        locked: Some(true),
        media_info: None,
        media_state: None,
        notifications_supported: Some(true),
        odometer: Some(50000.0),
        parsed_calendar_supported: Some(true),
        pf: Some(0),
        pr: Some(0),
        rd_window: Some(0),
        remote_start: Some(false),
        remote_start_enabled: Some(true),
        remote_start_supported: Some(true),
        rp_window: Some(0),
        rt: Some(0),
        santa_mode: Some(0),
        sentry_mode: Some(true),
        sentry_mode_available: Some(true),
        service_mode: Some(false),
        service_mode_plus: Some(false),
        smart_summon_available: Some(true),
        software_update: None,
        speed_limit_mode: None,
        summon_standby_mode_enabled: Some(false),
        timestamp: Some(timestamp),
        tpms_hard_warning_fl: Some(false),
        tpms_hard_warning_fr: Some(false),
        tpms_hard_warning_rl: Some(false),
        tpms_hard_warning_rr: Some(false),
        tpms_last_seen_pressure_time_fl: Some((timestamp / 1000) as i32),
        tpms_last_seen_pressure_time_fr: Some((timestamp / 1000) as i32),
        tpms_last_seen_pressure_time_rl: Some((timestamp / 1000) as i32),
        tpms_last_seen_pressure_time_rr: Some((timestamp / 1000) as i32),
        tpms_pressure_fl: Some(3.0),
        tpms_pressure_fr: Some(2.8),
        tpms_pressure_rl: Some(2.9),
        tpms_pressure_rr: Some(3.1),
        tpms_rcp_front_value: Some(3.1),
        tpms_rcp_rear_value: Some(3.1),
        tpms_soft_warning_fl: Some(false),
        tpms_soft_warning_fr: Some(false),
        tpms_soft_warning_rl: Some(false),
        tpms_soft_warning_rr: Some(false),
        valet_mode: Some(false),
        valet_pin_needed: Some(true),
        vehicle_name: Some("My Tesla".into()),
        vehicle_self_test_progress: Some(0),
        vehicle_self_test_requested: Some(false),
        webcam_available: Some(true),
    }
}

pub fn get_data(timestamp: DateTime<Utc>) -> VehicleData {
    let timestamp = timestamp.timestamp_millis() as u64;
    VehicleData {
        id: Some(1234567890123456),
        user_id: Some(123456),
        vehicle_id: Some(1),
        vin: Some("EWABCD123UWE23456".into()),
        display_name: Some("My Tesla".into()),
        option_codes: Some("AB12,HGF3".into()),
        color: None,
        access_type: None,
        tokens: Some(vec!["ys9wjhdow8djuwn8".into(), "mjsje8iuoshnvsoi".into()]),
        state: Some("online".into()),
        in_service: Some(false),
        id_s: Some("1273618746387526".into()),
        calendar_enabled: Some(true),
        api_version: Some(20),
        backseat_token: None,
        backseat_token_updated_at: None,
        ble_autopair_enrolled: None,
        charge_state: Some(get_charge_state(timestamp)),
        climate_state: Some(get_climate_state(timestamp)),
        drive_state: Some(get_drive_state(timestamp)),
        gui_settings: Some(get_gui_settings(timestamp)),
        vehicle_config: Some(get_vehicle_config(timestamp)),
        vehicle_state: Some(get_vehicle_state(timestamp)),
    }
}

// Create a VehicleData with the provided shift state
pub fn data_with_shift(timestamp: DateTime<Utc>, shift: Option<ShiftState>) -> VehicleData {
    let data = get_data(timestamp);
    VehicleData {
        drive_state: Some(DriveState {
            latitude: Some(12.34),
            longitude: Some(34.56),
            shift_state: shift,
            timestamp: Some(timestamp.timestamp_millis() as u64),
            ..data.drive_state.clone().unwrap()
        }),
        ..data
    }
}

pub fn data_with_state(timestamp: DateTime<Utc>, state: StateStatus) -> VehicleData {
    let mut data = get_data(timestamp);
    data.state = Some(state.as_str().to_string());
    data
}

pub fn data_charging(timestamp: DateTime<Utc>, batt_leval: i16) -> VehicleData {
    let mut data = get_data(timestamp);
    data.charge_state.as_mut().unwrap().battery_level = Some(batt_leval);
    data.charge_state.as_mut().unwrap().charging_state = Some(ChargingState::Charging);
    data.drive_state.as_mut().unwrap().shift_state = Some(ShiftState::P);
    data
}