use actix_web::HttpResponse;
use bigneon_db::utils::migration;
use crate::db::Connection;
use diesel::PgConnection;
use crate::errors::*;
use log::Level::*;

static mut IS_OK: bool = false;

pub fn check(connection: Connection) -> Result<HttpResponse, BigNeonError> {
    if unsafe { IS_OK } {
        return Ok(HttpResponse::Ok().finish());
    }

    let conn = connection.get();

    if let Err(err) = check_migrations(conn) {
        jlog!(Error, "bigneon::migrations", &err.reason, {});
        return Ok(HttpResponse::InternalServerError().finish());
    }

    // We only want to query migrations until it passes once
    unsafe {
        IS_OK = true;
    }
    Ok(HttpResponse::Ok().finish())
}

fn check_migrations(conn: &PgConnection) -> Result<(), ApplicationError> {
    migration::has_pending_migrations(conn)
        .map_err(|_err| ApplicationError::new("Error while checking migrations".to_string()))
        .and_then(|has_pending| {
            if has_pending {
                Err(ApplicationError::new("Migrations need to be run".to_string()))
            } else {
                Ok(())
            }
        })
}
