SHELL := /bin/bash

test_prepare:
	# docker-compose -f docker-compose.test.yml exec server_test diesel migration run --migration-dir=db/migrations
	DATABASE_URL=postgres://root@localhost:5433/sc_predictions_test diesel migration run --migration-dir=db/migrations

test:
	docker-compose -f docker-compose.test.yml exec database_test psql -d sc_predictions_test --c="TRUNCATE game_questions, user_questions, users, rounds, games, questions"
	# docker-compose -f docker-compose.test.yml exec server_test cargo test $(T) -- --nocapture --test-threads=1
	DATABASE_URL=postgres://root@localhost:5433/sc_predictions_test \
		CLIENT_HOST=http://localhost:3000 RUST_BACKTRACE=full \
		RUST_LOG="actix_web=debug" \
		JWT_KEY=77397A244326452948404D635166546A576E5A7234753778214125442A472D4A \
		cargo run --bin server


seed_files := $(wildcard seeds/*)
seed:
	$(foreach file,$(seed_files),docker-compose exec database psql -U root -d sc_predictions -a -f $(file))

run_server:
	DATABASE_URL=postgres://root@localhost:5432/sc_predictions \
		CLIENT_HOST=http://localhost:3000 RUST_BACKTRACE=full \
		RUST_LOG="actix_web=debug" \
		JWT_KEY=77397A244326452948404D635166546A576E5A7234753778214125442A472D4A \
		cargo run --bin server

.PHONY: seed test test_prepare run_server
