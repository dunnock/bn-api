#!/usr/bin/env bash

curl 'https://api.develop.bigneon.com/users/register' -H 'Accept: application/json, text/plain, */*' -H 'User-Agent: curl/memory leak test' -H 'Content-Type: application/json' -H 'Referer: https://develop.bigneon.com/sign-up' --data-binary '{"first_name":"Memory","last_name":"Leak","email":"test+memory+leak+account@tari.com","password":"h1uB8aO78Jpy"}'

ab -c 10 -n 1000000 -k -p ./scripts/leak_post_data.json -H 'User-Agent: ab/memory leak test' -T 'application/json' 'https://api.develop.bigneon.com/auth/token'