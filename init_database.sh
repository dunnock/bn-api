#!/usr/bin/env bash

# Get the directory where this script is. 
BN_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
DB_DIR=$BN_DIR/db
API_DIR=$BN_DIR/api

# Create .env files for the db and api projects
if [ ! -f $DB_DIR/.env ]; then # if file does not exist
  cp $DB_DIR/.env.sample $DB_DIR/.env
fi

if [ ! -f $API_DIR/.env ]; then # if file does not exist
  cp $API_DIR/.env.sample $API_DIR/.env
fi
cd $DB_DIR

# start Postgres using Docker, data is stored in db/pg_data
docker run -d -v pg_data:/var/lib/postgresql/data -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432/tcp postgres:latest

# wait till postgres has started
sleep 1 
while ! curl http://127.0.0.1:5432/ 2>&1 | grep '52'
do
  sleep 1 
done

# Create all the enviromental variable's for the database
set -o allexport
source .env
set +o allexport

# Creates the rust models in db/src/schema.rs and run the migrations
# diesel -V 2> /dev/null || cargo install diesel_cli --no-default-features --features postgres

# # Creates unit test database
# diesel database reset --database-url=$TEST_DATABASE_ADMIN_URL
# diesel setup --database-url=$TEST_DATABASE_ADMIN_URL
# diesel migration run --database-url=$TEST_DATABASE_ADMIN_URL

# # Concatenates all functions(stored procuderes) into functions.sql and runs them
# # They are mainly used for reports. 
# # Creates the test database
# cargo run functions -c $TEST_DATABASE_ADMIN_URL

# Creates the main database
cargo run create -c $TEST_DATABASE_ADMIN_URL -f -e superuser@test.com -p password -m 8883
cargo run create -c $DATABASE_ADMIN_URL -f -e superuser@test.com -p password -m 8883
