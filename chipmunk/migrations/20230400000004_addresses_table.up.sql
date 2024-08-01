CREATE TABLE public.addresses (
    id SERIAL PRIMARY KEY,
    display_name character varying(512),
    latitude FLOAT8,
    longitude FLOAT8,
    name character varying(255),
    house_number character varying(255),
    road character varying(255),
    neighbourhood character varying(255),
    city character varying(255),
    county character varying(255),
    postcode character varying(255),
    state character varying(255),
    state_district character varying(255),
    country character varying(255),
    raw jsonb,
    inserted_at timestamp(0) with time zone NOT NULL,
    updated_at timestamp(0) with time zone NOT NULL,
    osm_id bigint,
    osm_type text
);

CREATE UNIQUE INDEX addresses_osm_id_osm_type_index ON public.addresses USING btree (osm_id, osm_type);
