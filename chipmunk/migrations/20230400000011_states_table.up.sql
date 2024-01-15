CREATE TABLE public.states (
    id SERIAL PRIMARY KEY,
    state public.states_status NOT NULL,
    start_date timestamp without time zone NOT NULL,
    end_date timestamp without time zone,
    car_id smallint NOT NULL,
    CONSTRAINT positive_duration CHECK ((end_date >= start_date))
);

-- CREATE UNIQUE INDEX "states_car_id__end_date_IS_NULL_index" ON public.states USING btree (car_id, ((end_date IS NULL))) WHERE (end_date IS NULL);

ALTER TABLE ONLY public.states
    ADD CONSTRAINT states_car_id_fkey FOREIGN KEY (car_id) REFERENCES public.cars(id) ON DELETE CASCADE;
