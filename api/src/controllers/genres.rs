use actix_web::HttpResponse;
use bigneon_db::models::*;
use crate::db::Connection;
use crate::errors::*;
use uuid::Uuid;

pub fn index(connection: Connection) -> Result<HttpResponse, BigNeonError> {
    let genres = Genre::all(connection.get())?;
    Ok(HttpResponse::Ok()
        .json(json!({"genres": &genres.into_iter().map(|g| GenreListItem{id: g.id, name: g.name }).collect::<Vec<GenreListItem >>()})))
}

#[derive(Serialize)]
pub struct GenreListItem {
    pub id: Uuid,
    pub name: String,
}
