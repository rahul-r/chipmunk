CREATE TABLE public.charges (
    id SERIAL PRIMARY KEY,
    date TIMESTAMP WITH TIME ZONE NOT NULL,
    battery_heater_on BOOLEAN,
    battery_level SMALLINT,
    charge_energy_added FLOAT4,
    charger_actual_current SMALLINT,
    charger_phases SMALLINT,
    charger_pilot_current SMALLINT,
    charger_power SMALLINT NOT NULL,
    charger_voltage SMALLINT,
    fast_charger_present BOOLEAN,
    conn_charge_cable CHARACTER VARYING(255),
    fast_charger_brand CHARACTER VARYING(255),
    fast_charger_type CHARACTER VARYING(255),
    ideal_battery_range_km FLOAT4,
    not_enough_power_to_heat BOOLEAN,
    outside_temp FLOAT4,
    charging_process_id INT4 NOT NULL,
    battery_heater BOOLEAN,
    battery_heater_no_power BOOLEAN,
    rated_battery_range_km FLOAT4,
    usable_battery_level SMALLINT
);

ALTER TABLE ONLY public.charges
    ADD CONSTRAINT charges_charging_process_id_fkey FOREIGN KEY (charging_process_id) REFERENCES public.charging_processes(id) ON DELETE CASCADE;
