CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE games (
  id SERIAL PRIMARY KEY,
  locked BOOLEAN NOT NULL DEFAULT 'f',
  slug VARCHAR(10),
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  creator UUID NOT NULL DEFAULT uuid_generate_v4()
);

SELECT diesel_manage_updated_at('games');
