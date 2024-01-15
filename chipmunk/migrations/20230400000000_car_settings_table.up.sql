CREATE TABLE public.car_settings (
    id BIGINT PRIMARY KEY DEFAULT 1,
    suspend_min integer DEFAULT 21 NOT NULL,
    suspend_after_idle_min integer DEFAULT 15 NOT NULL,
    req_not_unlocked boolean DEFAULT false NOT NULL,
    free_supercharging boolean DEFAULT false NOT NULL,
    use_streaming_api boolean DEFAULT true NOT NULL,
    constraint one_row_only CHECK (id = 1)
);
