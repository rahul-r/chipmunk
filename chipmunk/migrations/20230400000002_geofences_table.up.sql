CREATE TABLE public.geofences (
    id SERIAL PRIMARY KEY,
    name character varying(255) NOT NULL,
    latitude FLOAT4 NOT NULL,
    longitude FLOAT4 NOT NULL,
    radius smallint DEFAULT 25 NOT NULL,
    inserted_at timestamp(0) without time zone NOT NULL,
    updated_at timestamp(0) without time zone NOT NULL,
    cost_per_unit FLOAT4,
    session_fee FLOAT4,
    billing_type public.billing_type DEFAULT 'per_kwh'::public.billing_type NOT NULL
);
