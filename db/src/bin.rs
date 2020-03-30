// Quiet diesel warnings https://github.com/diesel-rs/diesel/issues/1785
#![allow(proc_macro_derive_resolution_fallback)]
// Force these as errors so that they are not lost in all the diesel warnings
#![deny(unreachable_patterns)]
#![deny(unknown_lints)]
#![deny(unused_variables)]
#![deny(unused_imports)]
// Unused results is more often than not an error
#![deny(unused_must_use)]
#![deny(unused_extern_crates)]

#[macro_use]
extern crate diesel_migrations;
extern crate bigneon_db;
extern crate chrono;
extern crate clap;
extern crate diesel;

#[allow(unused_imports)]
embed_migrations!("./migrations");

use bigneon_db::prelude::*;
use chrono::prelude::Utc;
use clap::ArgMatches;
use clap::{App, Arg, SubCommand};
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::Connection;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

pub fn main() {
    let matches = App::new("Big Neon DB CLI")
        .author("Big Neon")
        .about("Command Line Interface for creating and migrating the Big Neon database")
        .subcommand(
            SubCommand::with_name("migrate")
                .about("Migrates the database to the latest version")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                ),
        )
        .subcommand(
            SubCommand::with_name("functions")
                .about("Runs the functions.sql file")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                ),
        )
        .subcommand(
            SubCommand::with_name("create")
                .about("Creates a new instance of the database and inserts the system administrator user")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                )
                .arg(
                    Arg::with_name("email")
                        .short("e")
                        .takes_value(true)
                        .help("email for system administrator"),
                )
                .arg(
                    Arg::with_name("phone")
                        .short("m")
                        .takes_value(true)
                        .help("phone number for system administrator"),
                )
                .arg(
                    Arg::with_name("password")
                        .short("p")
                        .takes_value(true)
                        .help("password for system administrator"),
                )
                .arg(
                    Arg::with_name("force")
                        .short("f")
                        .help("Drops the database if it exists. WARNING! This is NOT REVERSIBLE"),
                ),
        )
        .subcommand(
            SubCommand::with_name("drop")
                .about("Deletes the current database. WARNING! This is NOT REVERSIBLE")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                ),
        )
        .subcommand(
            SubCommand::with_name("new-migration")
                .about("Create a new migration")
                .arg(
                    Arg::with_name("name")
                        .long("name")
                        .short("n")
                        .takes_value(true)
                        .help("Name of the migration"),
                ),
        )
        .subcommand(
            SubCommand::with_name("rollback")
                .about("Rolls back the last migration")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                ),
        )
        .subcommand(
            SubCommand::with_name("seed")
                .about("Populates the database with example data")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                ),
        )
        .subcommand(
            SubCommand::with_name("user")
                .about("Creates an admin user")
                .arg(
                    Arg::with_name("connection")
                        .short("c")
                        .takes_value(true)
                        .help("Connection string to the database"),
                )
                .arg(
                    Arg::with_name("first")
                        .short("f")
                        .takes_value(true)
                        .help("first name for user"),
                )
                .arg(
                    Arg::with_name("last")
                        .short("l")
                        .takes_value(true)
                        .help("last name for user"),
                )
                .arg(
                    Arg::with_name("email")
                        .short("e")
                        .takes_value(true)
                        .help("email for user"),
                )
                .arg(
                    Arg::with_name("phone")
                        .short("m")
                        .takes_value(true)
                        .help("phone number user"),
                )
                .arg(
                    Arg::with_name("password")
                        .short("p")
                        .takes_value(true)
                        .help("password for user"),
                )
                .arg(
                    Arg::with_name("super")
                        .short("s")
                        .takes_value(false)
                        .help("Is the user a Super admin user"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("create", Some(matches)) => create_db_and_user(matches),
        ("user", Some(matches)) => create_user_only(matches),
        ("drop", Some(matches)) => drop_db(matches),
        ("migrate", Some(matches)) => migrate_db(matches),
        ("functions", Some(matches)) => run_function_migrations(matches),
        ("rollback", Some(matches)) => rollback_db(matches),
        ("new-migration", Some(matches)) => create_new_migration(matches),
        ("seed", Some(matches)) => seed_db(matches),
        _ => {
            eprintln!("Invalid subcommand '{}'", matches.subcommand().0);
        }
    }
}

fn change_database(conn_string: &str) -> String {
    let last_slash = conn_string
        .rfind('/')
        .expect("Connection string does not conform to <hostname>/<database>");
    format!("{}/postgres", conn_string.get(..last_slash).unwrap())
}

fn get_db(conn_string: &str) -> (String, String) {
    let parts: Vec<&str> = conn_string.split('/').collect();
    let db = parts.last().unwrap();
    let db = str::replace(db, "'", "''");
    let postgres_conn_string = change_database(conn_string);
    (postgres_conn_string, db)
}

fn execute_sql(postgres_conn_string: &str, query: &str) -> Result<(), diesel::result::Error> {
    let connection = PgConnection::establish(&postgres_conn_string).unwrap();
    connection.execute(query).map(|_i| ())
}

fn create_db(conn_string: &str) -> Result<(), diesel::result::Error> {
    let (postgres_conn_string, db) = get_db(conn_string);
    execute_sql(&postgres_conn_string, &format!("CREATE DATABASE \"{}\"", db))
}

fn migrate_db(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");

    match create_db(conn_string) {
        Ok(_o) => println!("Creating database"),
        Err(_e) => println!("Database already exists"),
    }
    println!("Migrating database");

    let connection = PgConnection::establish(conn_string).unwrap();

    embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).expect("Migration failed");

    run_function_migrations(matches);
}

fn run_function_migrations(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");
    println!("Running functions.sql");
    let functions_query = include_str!("../functions/functions.sql");

    let db_connection = get_connection(conn_string);

    db_connection
        .batch_execute(functions_query)
        .expect("Functions query failed");
}

fn rollback_db(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");

    println!("Rollback database");

    let connection = PgConnection::establish(conn_string).unwrap();

    match diesel_migrations::revert_latest_migration(&connection) {
        Ok(s) => std::io::stdout().write(s.as_bytes()),
        Err(e) => std::io::stderr().write(e.to_string().as_bytes()),
    }
    .expect("Rollback failed");
}

fn create_db_and_user(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");

    if matches.is_present("force") {
        drop_db(matches);
    }

    create_db(conn_string).expect("Can't create database because one with the same name already exists");

    {
        let connection = get_connection(conn_string);

        embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).expect("Migration failed");

        run_function_migrations(matches);
    }

    let email = matches.value_of("email").expect("Email was not provided");
    let phone = matches.value_of("phone").expect("Phone number was not provided");
    let password = matches.value_of("password").expect("Password was not provided");

    let db_connection = get_connection(conn_string);
    create_user(
        "System".to_string(),
        "Administrator".to_string(),
        email.to_string(),
        phone.to_string(),
        password,
        true,
        &db_connection,
    );
}

fn create_user_only(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");

    let first = matches.value_of("first").expect("First name was not provided");
    let last = matches.value_of("last").expect("Last name was not provided");
    let email = matches.value_of("email").expect("Email was not provided");
    let phone = matches.value_of("phone").expect("Phone number was not provided");
    let password = matches.value_of("password").expect("Password was not provided");

    let db_connection = get_connection(conn_string);
    create_user(
        first.to_string(),
        last.to_string(),
        email.to_string(),
        phone.to_string(),
        password,
        matches.is_present("super"),
        &db_connection,
    );
}

fn create_user(
    first_name: String,
    last_name: String,
    email: String,
    phone: String,

    password: &str,
    is_super: bool,
    db_connection: &PgConnection,
) {
    println!("Creating user");
    let user = User::create(Some(first_name), Some(last_name), Some(email), Some(phone), &password)
        .commit(None, &db_connection)
        .expect("Failed to create system admin");
    let user = user
        .add_role(Roles::Admin, &db_connection)
        .expect("Could not assign System Administrator role to the user");
    if is_super {
        user.add_role(Roles::Super, &db_connection)
            .expect("Could not assign System Administrator role to the user");
    }
}

fn seed_db(matches: &ArgMatches) {
    println!("Seeding database");
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");

    let db_connection = get_connection(conn_string);

    let seed_query = include_str!("seed_data/seed.sql");
    println!("Seed {}", seed_query);

    db_connection
        .batch_execute(seed_query)
        .expect("Seeding database failed");
}

fn get_connection(connection_string: &str) -> PgConnection {
    PgConnection::establish(&connection_string).expect("Error connecting to DB")
}

fn drop_db(matches: &ArgMatches) {
    let conn_string = matches
        .value_of("connection")
        .expect("Connection string was not provided");
    let (postgres_conn_string, db) = get_db(conn_string);
    println!("Dropping {} from {}", db, postgres_conn_string);
    execute_sql(&postgres_conn_string, &format!("DROP DATABASE IF EXISTS \"{}\"", db))
        .expect("Error dropping database");
}

fn create_new_migration(matches: &ArgMatches) {
    let name = matches.value_of("name").expect("Expected migration name");

    let name = name.replace(" ", "_").to_ascii_lowercase();
    let timestamp = Utc::now().format("%Y%m%d%H%M%S");

    let dir_name = format!("{}_{}", timestamp, name);

    println!("Creating migration '{}'", dir_name);

    let migration_dir = Path::new("./migrations").join(dir_name);
    create_dir_all(&migration_dir).expect("Error creating migration directory");

    let up_path = migration_dir.join("up.sql");
    let up_path = up_path.to_str().expect("Error converting path to string");
    println!("Creating {}...", up_path);
    File::create(up_path).expect("Error creating migration file");

    let down_path = migration_dir.join("down.sql");
    let down_path = down_path.to_str().expect("Error converting path to string");
    println!("Creating {}...", down_path);
    File::create(down_path).expect("Error creating migration file");
}
