SHELL := /bin/bash

test_prepare:
	docker-compose -f docker-compose.test.yml exec database_test dropdb sc_predictions_test
	docker-compose -f docker-compose.test.yml exec database_test createdb sc_predictions_test
	docker-compose -f docker-compose.test.yml exec server_test dbmigrate up

test:
	docker-compose -f docker-compose.test.yml exec server_test cargo test $(T) -- --test-threads=1


seed_files := $(wildcard seeds/*)
seed:
	$(foreach file,$(seed_files),docker-compose exec database psql -U root -d sc_predictions -a -f $(file))


.PHONY: seed test test_prepare
