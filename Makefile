SHELL := /bin/bash

test_prepare:
	docker-compose -f docker-compose.test.yml exec server_test diesel migration run --migration-dir=db/migrations

test:
	docker-compose -f docker-compose.test.yml exec database_test psql -d sc_predictions_test --c="TRUNCATE game_questions, user_questions, users, rounds, games, questions"
	docker-compose -f docker-compose.test.yml exec server_test cargo test $(T) -- --nocapture --test-threads=1


seed_files := $(wildcard seeds/*)
seed:
	$(foreach file,$(seed_files),docker-compose exec database psql -U root -d sc_predictions -a -f $(file))


.PHONY: seed test test_prepare
