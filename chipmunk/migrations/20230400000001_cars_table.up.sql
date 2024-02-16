CREATE TABLE public.cars (
    id SMALLSERIAL PRIMARY KEY,
    eid bigint NOT NULL UNIQUE,
    vid bigint NOT NULL UNIQUE,
    model character varying(255),
    efficiency FLOAT4,
    inserted_at timestamp(0) without time zone NOT NULL,
    updated_at timestamp(0) without time zone NOT NULL,
    vin text UNIQUE,
    name text,
    trim_badging text,
    settings_id bigint NOT NULL,
    exterior_color text,
    spoiler_type text,
    wheel_type text,
    display_priority smallint DEFAULT 1 NOT NULL,
    marketing_name character varying(255)
);

ALTER TABLE ONLY public.cars
    ADD CONSTRAINT cars_settings_id_fkey FOREIGN KEY (settings_id) REFERENCES public.car_settings(id) ON DELETE CASCADE;
