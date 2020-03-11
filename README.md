# Big Neon API

[![Build Status](https://drone.metalworks.tarilabs.com/api/badges/big-neon/bn-api/status.svg)](https://drone.metalworks.tarilabs.com/big-neon/bn-api)
[![Docker Repository on Quay](https://quay.io/repository/tarilabs/bn-api/status "Docker Repository on Quay")](https://quay.io/repository/tarilabs/bn-api)

This is the Big Neon API repository, part of the Big Neon ticketing system.

# Overall project architecture

Big Neon is a multi-tiered MVC Web API architecture for selling and managing tickets. The software system is made up 
of multiple components. To get an overall picture of how 
everything fits together have a look at the [docs repository]( https://github.com/big-neon/docs.git)

# Building this project from source

To download and build this project, 

1. Clone the source

        git clone https://github.com/big-neon/bn-api.git
    
1. Compile
        
        cargo build
        
# Configuring the local development environment

## 1. Update submodules

```
git submodule init
git submodule update
```

## 2. Initialize database and redis

```
source init_database.sh
docker run --name bigneon_redis -d redis
```

## 3. Start Bigneon API

```
cd api; cargo run --bin server
```

# Setting up Facebook login
Facebook login is optional. If you would like to enable it, you will need to get
an `app id` via the Facebook Developer page.

On the Facebook Developer page, you will also need to set the `Valid  OAuth Redirect URIs`.

# Workspace crates

- payments - payments processing handlers
- locators - services locator and deep links factory
- errors - error types
- domain_events - background processing of bigneon events
- db_connectors - database connection handlers
- config - application configuration
- communications - external communication via mailers, pushers and smsers
- api - main server