SHELL := /bin/bash

db_url := postgres://postgres:abc123@localhost:5434/sc_predictions

test_prepare:
	# docker-compose -f docker-compose.test.yml exec server_test diesel migration run --migration-dir=db/migrations
	DATABASE_URL=postgres://root@localhost:5433/sc_predictions_test diesel migration run --migration-dir=db/migrations

test:
	docker-compose -f docker-compose.test.yml exec database_test psql -d sc_predictions_test --c="TRUNCATE game_questions, user_questions, users, rounds, games, questions"
	# docker-compose -f docker-compose.test.yml exec server_test cargo test $(T) -- --nocapture --test-threads=1
	DATABASE_URL=postgres://root@localhost:5433/sc_predictions_test \
		CLIENT_HOST=http://localhost:3000 RUST_BACKTRACE=full \
		JWT_KEY=77397A244326452948404D635166546A576E5A7234753778214125442A472D4A \
		cargo test $(T) -- --nocapture --test-threads=1


seeds:
	DATABASE_URL=$(db_url) cargo run --bin seeds

create_migration:
	DATABASE_URL=$(db_url) diesel migration generate $(name) --migration-dir=db/migrations

migrate:
	DATABASE_URL=$(db_url) diesel migration run --migration-dir=db/migrations

redo_migrate:
	DATABASE_URL=$(db_url) diesel migration redo --migration-dir=db/migrations

run_server:
	DATABASE_URL=$(db_url) \
		CLIENT_HOST=http://localhost:3000 RUST_BACKTRACE=full \
		JWT_KEY=77397A244326452948404D635166546A576E5A7234753778214125442A472D4A \
		cargo run --bin server

.PHONY: seeds test test_prepare run_server
