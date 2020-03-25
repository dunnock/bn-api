#!/usr/bin/env bash

./target/release/bndb_cli create -c $DATABASE_URL -f -e superuser@test.com -p password -m 8883 || {
    echo "Migrations failed"
    exit 1
}

./target/release/server -t false -r false &> /tmp/api.log &
export SERVER_PID=$!$1

# Run newman tests
#newman run --timeout-request 60000 ./integration-tests/bigneon-tests.postman_collection.json -e ./integration-tests/travis.postman_environment.json

cd ./integration-tests/mocha
npm install && npm test

NEWMAN_EXIT_CODE=$?
kill -s SIGTERM $SERVER_PID

if [[ $NEWMAN_EXIT_CODE -ne 0 ]]
then
    cat /tmp/api.log
    exit $NEWMAN_EXIT_CODE
fi

cd ../../

./target/release/server -b true || {
    echo "Tasks failed"
    exit 1
}

