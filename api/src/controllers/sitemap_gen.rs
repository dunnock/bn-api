use crate::database::Connection;
use crate::errors::ApiError;
use crate::server::AppState;
use crate::utils::gen_sitemap;
use actix_web::{web::Data, HttpResponse};

pub async fn index((connection, state): (Connection, Data<AppState>)) -> Result<HttpResponse, ApiError> {
    let conn = connection.get();

    let sitemap_xml = gen_sitemap::create_sitemap_conn(conn, &state.config.front_end_url)?;

    Ok(HttpResponse::Ok().content_type("text/xml").body(sitemap_xml).into())
}
