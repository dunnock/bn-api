#!/usr/bin/env bash
cd ../db
cargo run create -c $DATABASE_URL -f -e superuser@test.com -p password -m 8883
cd ../api && cargo test
