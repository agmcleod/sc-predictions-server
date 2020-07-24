-- This file should undo anything in `up.sql`
ALTER TABLE game_questions ALTER COLUMN game_id DROP NOT NULL;
ALTER TABLE game_questions ALTER COLUMN question_id DROP NOT NULL;