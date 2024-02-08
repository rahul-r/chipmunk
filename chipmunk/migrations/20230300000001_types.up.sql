CREATE TYPE public.unit_of_length AS ENUM
    ('km', 'mi');

CREATE TYPE public.unit_of_pressure AS ENUM
    ('bar', 'psi');

CREATE TYPE public.unit_of_temperature AS ENUM
    ('C', 'F');

CREATE TYPE public.billing_type AS ENUM
    ('per_kwh', 'per_minute');

-- CREATE TYPE public.cube
-- (
--     INPUT = cube_in,
--     OUTPUT = cube_out,
--     RECEIVE = cube_recv,
--     SEND = cube_send,
--     INTERNALLENGTH = -1,
--     ALIGNMENT =  double,
--     STORAGE =  PLAIN,
--     CATEGORY = 'U',
--     DELIMITER = ','
-- );

-- COMMENT ON TYPE public.cube
--     IS 'multi-dimensional cube ''(FLOAT-1, FLOAT-2, ..., FLOAT-N), (FLOAT-1, FLOAT-2, ..., FLOAT-N)''';

CREATE TYPE public.range AS ENUM
    ('ideal', 'rated');

CREATE TYPE public.states_status AS ENUM
    ('offline', 'asleep', 'unknown', 'parked', 'driving', 'charging');

CREATE TYPE public.charge_stat AS ENUM
    ('start', 'stop', 'charging', 'done', 'idle');