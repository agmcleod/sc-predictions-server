# Starcraft 2 predictions app

## Running Migrations

Using [https://github.com/Keats/dbmigrate](https://github.com/Keats/dbmigrate)

```
# create a migration file
docker-compose exec server dbmigrate create my_name
# apply all non applied migrations
docker-compose exec server dbmigrate up
# un-apply all migrations
docker-compose exec server dbmigrate down
# redo the last migration
docker-compose exec server dbmigrate redo
# revert the last migration
docker-compose exec server dbmigrate revert
# see list of migrations and which one is currently applied
docker-compose exec server dbmigrate status
```