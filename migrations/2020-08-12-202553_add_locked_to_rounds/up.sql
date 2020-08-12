-- Your SQL goes here
ALTER TABLE rounds
    ADD COLUMN locked BOOLEAN NOT NULL DEFAULT 'f';