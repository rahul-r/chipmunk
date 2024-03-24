CREATE TABLE public.charging_processes (
    id SERIAL PRIMARY KEY,
    start_date timestamp with time zone NOT NULL,
    end_date timestamp with time zone,
    charge_energy_added FLOAT4,
    start_ideal_range_km FLOAT4,
    end_ideal_range_km FLOAT4,
    start_battery_level smallint,
    end_battery_level smallint,
    duration_min smallint,
    outside_temp_avg FLOAT4,
    car_id smallint NOT NULL,
    position_id integer NOT NULL,
    address_id integer,
    start_rated_range_km FLOAT4,
    end_rated_range_km FLOAT4,
    geofence_id integer,
    charge_energy_used FLOAT4,
    cost FLOAT4,
    charging_status public.charge_stat NOT NULL
);

ALTER TABLE ONLY public.charging_processes
    ADD CONSTRAINT charging_processes_address_id_fkey FOREIGN KEY (address_id) REFERENCES public.addresses(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.charging_processes
    ADD CONSTRAINT charging_processes_car_id_fkey FOREIGN KEY (car_id) REFERENCES public.cars(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.charging_processes
    ADD CONSTRAINT charging_processes_geofence_id_fkey FOREIGN KEY (geofence_id) REFERENCES public.geofences(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.charging_processes
    ADD CONSTRAINT charging_processes_position_id_fkey FOREIGN KEY (position_id) REFERENCES public.positions(id);
