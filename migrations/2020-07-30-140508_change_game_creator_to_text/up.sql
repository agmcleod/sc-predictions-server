ALTER TABLE games
    ALTER COLUMN creator SET DATA TYPE TEXT,
    ALTER COLUMN creator DROP NOT NULL,
    ALTER COLUMN creator SET DEFAULT NULL;