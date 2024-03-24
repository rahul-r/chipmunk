CREATE TABLE public.settings (
    id BIGINT PRIMARY KEY DEFAULT 1,
    inserted_at timestamp(0) with time zone NOT NULL,
    updated_at timestamp(0) with time zone NOT NULL,
    unit_of_length public.unit_of_length DEFAULT 'km'::public.unit_of_length NOT NULL,
    unit_of_temperature public.unit_of_temperature DEFAULT 'C'::public.unit_of_temperature NOT NULL,
    preferred_range public.range DEFAULT 'rated'::public.range NOT NULL,
    base_url character varying(255),
    grafana_url character varying(255),
    language text DEFAULT 'en'::text NOT NULL,
    unit_of_pressure public.unit_of_pressure DEFAULT 'bar'::public.unit_of_pressure NOT NULL,
    logging_period_ms integer NOT NULL,
    log_at_startup boolean NOT NULL,
    constraint one_row_only CHECK (id = 1)
);
