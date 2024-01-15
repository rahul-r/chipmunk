CREATE TABLE public.tokens (
    id INTEGER PRIMARY KEY DEFAULT 1,
    refresh_token BYTEA,
    refresh_token_iv BYTEA,
    access_token BYTEA,
    access_token_iv BYTEA,
    access_token_expires_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    id_token BYTEA,
    id_token_iv BYTEA,
    token_type TEXT ,
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    constraint one_row_only CHECK (id = 1)
);
