-- Your SQL goes here
ALTER TABLE games
    ADD COLUMN locked BOOLEAN NOT NULL DEFAULT 'f';