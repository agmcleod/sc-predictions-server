ALTER TABLE users
    ALTER COLUMN session_id TYPE UUID,
    ALTER COLUMN session_id SET NOT NULL,
    ALTER COLUMN session_id SET DEFAULT uuid_generate_v4();
