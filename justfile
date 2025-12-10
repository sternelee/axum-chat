set dotenv-load


init:
	cargo install cargo-watch
	just db-migrate

dev-server:
	cargo watch -w src -w templates -w tailwind.config.js -w input.css -x run 

dev-tailwind:
	./tailwindcss -i input.css -o assets/output.css --watch=always

build-server:
	cargo build --release

build-tailwind:
	./tailwindcss -i input.css -o assets/output.css --minify


db-migrate:
  echo "Migrating ..."
  sqlite3 $DATABASE_PATH < db/migrations/20231101170247_init.sql
  sqlite3 $DATABASE_PATH < db/migrations/20241122000001_providers_agents.sql
  sqlite3 $DATABASE_PATH < db/migrations/20241123000001_add_allow_tools.sql

db-reset:
  echo "Resetting ..."
  rm -f $DATABASE_PATH
  sqlite3 $DATABASE_PATH < db/migrations/20231101170247_init.sql
  sqlite3 $DATABASE_PATH < db/migrations/20241122000001_providers_agents.sql
  sqlite3 $DATABASE_PATH < db/migrations/20241123000001_add_allow_tools.sql
  sqlite3 $DATABASE_PATH < seeds/seed-users.sql

dev:
	#!/bin/sh
	just dev-tailwind &
	pid1=$!
	just dev-server &
	pid2=$!
	trap "kill $pid1 $pid2" EXIT
	wait $pid1 $pid2
