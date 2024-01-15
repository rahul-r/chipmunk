CREATE TABLE public.updates (
    id SERIAL PRIMARY KEY,
    start_date timestamp without time zone NOT NULL,
    end_date timestamp without time zone,
    version character varying(255),
    car_id smallint NOT NULL,
    CONSTRAINT positive_duration CHECK ((end_date >= start_date))
);

ALTER TABLE ONLY public.updates
    ADD CONSTRAINT updates_car_id_fkey FOREIGN KEY (car_id) REFERENCES public.cars(id) ON DELETE CASCADE;
