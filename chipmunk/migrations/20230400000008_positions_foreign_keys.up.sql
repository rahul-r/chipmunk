ALTER TABLE ONLY public.positions
    ADD CONSTRAINT positions_car_id_fkey FOREIGN KEY (car_id) REFERENCES public.cars(id) ON DELETE CASCADE;

ALTER TABLE ONLY public.positions
    ADD CONSTRAINT positions_drive_id_fkey FOREIGN KEY (drive_id) REFERENCES public.drives(id) ON DELETE SET NULL;
