# Starcraft 2 predictions app

This idea came from an activity we did at a Toronto PubCraft, where we make predictions for certain things between two players in a single game. We tracked it on paper, which works fine, but why not something you can access with your phone?

I don't know if this is something I plan to finish. I'm not sure I want to worry about hosting it. Just a fun little thing for me to learn more about Rust in the web development arena.

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

## Running tests

```
dockr test up # make sure test server is running
make test_prepare
make test
```
