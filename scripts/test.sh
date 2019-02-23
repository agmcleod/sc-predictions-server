docker-compose -f docker-compose.test.yml exec server_test cargo test $1 -- --test-threads=1
