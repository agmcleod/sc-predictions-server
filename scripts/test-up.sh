docker-compose -f docker-compose.test.yml up -d
docker-compose -f docker-compose.test.yml exec database_test dropdb sc_predictions_test
docker-compose -f docker-compose.test.yml exec database_test createdb sc_predictions_test
docker-compose -f docker-compose.test.yml exec server_test dbmigrate up
