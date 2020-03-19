use crate::database::Connection;
use crate::errors::*;
use actix_web::HttpResponse;
use db::models::*;
use uuid::Uuid;

pub async fn index(connection: Connection) -> Result<HttpResponse, ApiError> {
    let genres = Genre::all(connection.get())?;
    Ok(HttpResponse::Ok()
        .json(json!({"genres": &genres.into_iter().map(|g| GenreListItem{id: g.id, name: g.name }).collect::<Vec<GenreListItem >>()})))
}

#[derive(Serialize)]
pub struct GenreListItem {
    pub id: Uuid,
    pub name: String,
}
