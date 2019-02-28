CREATE TABLE game_questions (
  id SERIAL PRIMARY KEY,
  game_id INTEGER REFERENCES games(id),
  question_id INTEGER REFERENCES questions(id),
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

SELECT manage_updated_at('game_questions');
