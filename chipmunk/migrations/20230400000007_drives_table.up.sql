CREATE TABLE public.drives (
    id SERIAL PRIMARY KEY,
    start_date timestamp without time zone NOT NULL,
    end_date timestamp without time zone,
    outside_temp_avg FLOAT4,
    speed_max FLOAT4,
    power_max FLOAT4,
    power_min FLOAT4,
    start_ideal_range_km FLOAT4,
    end_ideal_range_km FLOAT4,
    start_km FLOAT4,
    end_km FLOAT4,
    distance FLOAT4,
    duration_min smallint,
    car_id smallint NOT NULL,
    inside_temp_avg FLOAT4,
    start_address_id integer,
    end_address_id integer,
    start_rated_range_km FLOAT4,
    end_rated_range_km FLOAT4,
    start_position_id integer,
    end_position_id integer,
    start_geofence_id integer,
    end_geofence_id integer,
    status public.drive_stat NOT NULL
);

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_car_id_fkey FOREIGN KEY (car_id) REFERENCES public.cars(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_end_address_id_fkey FOREIGN KEY (end_address_id) REFERENCES public.addresses(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_end_geofence_id_fkey FOREIGN KEY (end_geofence_id) REFERENCES public.geofences(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_end_position_id_fkey FOREIGN KEY (end_position_id) REFERENCES public.positions(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_start_address_id_fkey FOREIGN KEY (start_address_id) REFERENCES public.addresses(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_start_geofence_id_fkey FOREIGN KEY (start_geofence_id) REFERENCES public.geofences(id) ON DELETE SET NULL;

ALTER TABLE ONLY public.drives
    ADD CONSTRAINT drives_start_position_id_fkey FOREIGN KEY (start_position_id) REFERENCES public.positions(id) ON DELETE SET NULL;
