use chrono::NaiveDateTime;
use diesel;
use diesel::prelude::*;
use schema::wallets;
use std::default::Default;
use tari_client::{convert_bytes_to_hexstring, cryptographic_keypair};
use utils::errors;
use utils::errors::*;
use uuid::Uuid;

#[derive(Identifiable, Queryable, Clone)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    name: String,
    pub secret_key: String,
    pub public_key: String,
    default_flag: bool,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

impl Wallet {
    pub fn find(id: Uuid, conn: &PgConnection) -> Result<Wallet, DatabaseError> {
        wallets::table
            .filter(wallets::id.eq(id))
            .first(conn)
            .to_db_error(errors::ErrorCode::QueryError, "Could not find wallet")
    }

    pub fn create_for_user(
        user_id: Uuid,
        name: String,
        force: bool,
        conn: &PgConnection,
    ) -> Result<Wallet, DatabaseError> {
        let (secret_key, public_key) = cryptographic_keypair();
        let mut default_flag: bool = true;
        if !force {
            let wallets = Wallet::find_for_user(user_id, conn)?;

            if wallets.len() == 0 {
                default_flag = true;
            } else {
                default_flag = false;
            };
        }

        NewWallet {
            user_id: Some(user_id),
            name,
            secret_key: convert_bytes_to_hexstring(&secret_key),
            public_key: convert_bytes_to_hexstring(&public_key),
            default_flag,
            ..Default::default()
        }
        .commit(conn)
    }

    pub fn create_for_organization(
        organization_id: Uuid,
        name: String,
        conn: &PgConnection,
    ) -> Result<Wallet, DatabaseError> {
        let (secret_key, public_key) = cryptographic_keypair();
        let wallets = Wallet::find_for_organization(organization_id, conn)?;
        let default_flag: bool;
        if wallets.len() == 0 {
            default_flag = true;
        } else {
            default_flag = false;
        };
        NewWallet {
            organization_id: Some(organization_id),
            name,
            secret_key: convert_bytes_to_hexstring(&secret_key),
            public_key: convert_bytes_to_hexstring(&public_key),
            default_flag,
            ..Default::default()
        }
        .commit(conn)
    }

    pub fn find_for_user(user_id: Uuid, conn: &PgConnection) -> Result<Vec<Wallet>, DatabaseError> {
        wallets::table
            .filter(wallets::user_id.eq(user_id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find wallets")
    }

    pub fn find_for_organization(organization_id: Uuid, conn: &PgConnection) -> Result<Vec<Wallet>, DatabaseError> {
        wallets::table
            .filter(wallets::organization_id.eq(organization_id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find wallets")
    }

    pub fn find_default_for_user(user_id: Uuid, conn: &PgConnection) -> Result<Wallet, DatabaseError> {
        let wallets = Wallet::find_for_user(user_id, conn)?;
        let mut result_wallet;
        if wallets.len() > 0 {
            result_wallet = wallets[0].clone(); //Return first wallet if no default wallet found
            for wallet in wallets {
                if wallet.default_flag {
                    result_wallet = wallet;
                    break;
                }
            }
        } else {
            //Create default wallet for user
            result_wallet = Wallet::create_for_user(user_id, "Default".to_string(), true, conn)?;
        }
        Ok(result_wallet)
    }

    pub fn find_default_for_organization(organization_id: Uuid, conn: &PgConnection) -> Result<Wallet, DatabaseError> {
        let wallets = Wallet::find_for_organization(organization_id, conn)?;
        let mut result_wallet;
        if wallets.len() > 0 {
            result_wallet = wallets[0].clone(); //Return first wallet if no default wallet found
            for wallet in wallets {
                if wallet.default_flag {
                    result_wallet = wallet;
                    break;
                }
            }
        } else {
            //Create default wallet for org
            result_wallet = Wallet::create_for_organization(organization_id, "Default".to_string(), conn)?;
        }
        Ok(result_wallet)
    }
}

#[derive(Default, Insertable)]
#[table_name = "wallets"]
pub struct NewWallet {
    user_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    name: String,
    secret_key: String,
    public_key: String,
    default_flag: bool,
}

impl NewWallet {
    pub fn commit(self, conn: &PgConnection) -> Result<Wallet, DatabaseError> {
        diesel::insert_into(wallets::table)
            .values(self)
            .get_result(conn)
            .to_db_error(ErrorCode::InsertError, "Could not create wallet")
    }
}
