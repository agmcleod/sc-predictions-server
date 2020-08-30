-- Your SQL goes here
CREATE TABLE rounds (
    id SERIAL PRIMARY KEY,
    player_one VARCHAR NOT NULL,
    player_two VARCHAR NOT NULL,
    game_id INTEGER NOT NULL REFERENCES games(id),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

SELECT diesel_manage_updated_at('rounds');