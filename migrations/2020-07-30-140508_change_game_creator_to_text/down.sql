ALTER TABLE games
    DROP COLUMN creator,
    ADD COLUMN creator UUID NOT NULL DEFAULT uuid_generate_v4();
