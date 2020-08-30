# Starcraft 2 predictions app

**Project Status**

At present not really working on this. I hit challenges on trying to make error handling + validations work. Feels like a lot of high friction. Actix has recently hit version 2, so I may try to revisit.

**Description**

This idea came from an activity we did at a Toronto PubCraft, where we make predictions for certain things between two players in a single game. We tracked it on paper, which works fine, but why not something you can access with your phone?

I don't know if this is something I plan to finish. I'm not sure I want to worry about hosting it. Just a fun little thing for me to learn more about Rust in the web development arena.

## Setup

Install docker.

Start up the app, will take time to compile:

```
docker-compose up
```

I use a shortcut command via bash. Can add to your .bashrc

```bash
dockr () {
      local override="docker-compose.$1.yml"
      if [ ! -f "$override" ]; then
          local override="docker-compose.yml"
      else
          shift
      fi
      docker-compose -f "$override" $*
}
```

Can then run `dockr up` instead of the above. The rest of the readme uses this for brevity.

## Running Migrations

(Optional) Install diesel-cli:

```bash
cargo install diesel_cli --no-default-features --features postgres
```

Can then use it locally, or run the cli via docker:

```
# create a migration file
dockr exec server diesel migration generate create_posts --migration-dir=db/migrations
# apply all non applied migrations
dockr exec server diesel migration run --migration-dir=db/migrations
# redo the last migration
dockr exec server diesel migration redo --migration-dir=db/migrations
```

## Seeds

```
dockr exec server cargo run --bin seeds
```

## Running tests

```
dockr test up # make sure test server is running
make test_prepare
make test
```

Can run tests by name via:

```
make test T=join_game
```
