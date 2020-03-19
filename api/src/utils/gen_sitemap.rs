use crate::errors::{ApiError, ApplicationError, ApplicationErrorType};
use chrono::Utc;
use db::prelude::*;
use diesel::PgConnection;
use itertools::Itertools;
use sitemap::structs::UrlEntry;
use sitemap::writer::SiteMapWriter;
use std::io::{Cursor, Read};
use uuid::Uuid;

pub fn create_sitemap(urls: &[String]) -> Result<String, ApiError> {
    let mut output = Cursor::new(Vec::new());
    {
        let sitemap_writer = SiteMapWriter::new(&mut output);

        let mut urlwriter = sitemap_writer.start_urlset().map_err(|_e| {
            ApplicationError::new_with_type(
                ApplicationErrorType::Internal,
                "fn create_sitemap: Unable to write urlset".to_string(),
            )
        })?;

        for url in urls.iter() {
            urlwriter.url(UrlEntry::builder().loc(url.clone())).map_err(|_e| {
                ApplicationError::new_with_type(
                    ApplicationErrorType::Internal,
                    format!("fn create_sitemap: Unable to write url, {}", url),
                )
            })?;
        }
        urlwriter.end().map_err(|_e| {
            ApplicationError::new_with_type(
                ApplicationErrorType::Internal,
                "fn create_sitemap: Unable to write close tags".to_string(),
            )
        })?;
    }
    let mut buffer = String::new();
    output.set_position(0);
    output.read_to_string(&mut buffer)?;
    Ok(buffer)
}

pub fn create_sitemap_conn(conn: &PgConnection, front_end_url: &String) -> Result<String, ApiError> {
    //find all active events
    let events_slug_ids = Event::find_all_active_events(conn)?
        .iter()
        .filter(|e| e.publish_date.is_some() && e.publish_date.unwrap() < Utc::now().naive_utc())
        .map(|e| e.slug_id)
        .filter(|s| s.is_some())
        .map(|s| s.unwrap())
        .collect_vec();

    let event_urls = create_urls(front_end_url, events_slug_ids, "tickets".to_string(), conn);

    // Cities
    let city_slug_id = Slug::find_by_slug_type(SlugTypes::City, conn)?
        .iter()
        .map(|s| s.id)
        .collect_vec();
    let city_urls = create_urls(front_end_url, city_slug_id, "cities".to_string(), conn);

    // Venues
    let venue_slugs_ids = Slug::find_by_slug_type(SlugTypes::Venue, conn)?
        .iter()
        .map(|s| s.id)
        .collect_vec();
    let venue_urls = create_urls(front_end_url, venue_slugs_ids, "venues".to_string(), conn);

    // Organizations
    let organizations_slugs_ids = Slug::find_by_slug_type(SlugTypes::Organization, conn)?
        .iter()
        .map(|s| s.id)
        .collect_vec();
    let organizations_urls = create_urls(
        front_end_url,
        organizations_slugs_ids,
        "organizations".to_string(),
        conn,
    );

    // Genres
    let genre_slug_ids = Slug::find_by_slug_type(SlugTypes::Genre, conn)?
        .iter()
        .map(|s| s.id)
        .collect_vec();
    let genre_urls = create_urls(front_end_url, genre_slug_ids, "genres".to_string(), conn);

    let mut urls = event_urls;
    urls.extend(venue_urls);
    urls.extend(organizations_urls);
    urls.extend(city_urls);
    urls.extend(genre_urls);

    let sitemap_xml = create_sitemap(&urls)?;
    Ok(sitemap_xml)
}

pub fn create_urls(front_url: &String, slug_ids: Vec<Uuid>, url_parm: String, conn: &PgConnection) -> Vec<String> {
    let slugs = Slug::find_all(slug_ids, conn).unwrap();
    let gen_urls = slugs
        .iter()
        .map(|slug| &slug.slug)
        .unique()
        .map(|s| format!("{}/{}/{}", front_url, url_parm, s))
        .collect_vec();
    gen_urls
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_sitemap_test() {
        let v = vec![
            "http://github.com".to_string(),
            "http://google.com".to_string(),
            "http://yandex.ru".to_string(),
        ];
        let buffer = create_sitemap(&v).unwrap();
        println!("result: {}", buffer);
        let result = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url>\n    <loc>http://github.com/</loc>\n  </url>\n  <url>\n    <loc>http://google.com/</loc>\n  </url>\n  <url>\n    <loc>http://yandex.ru/</loc>\n  </url>\n</urlset>";
        assert_eq!(buffer, result);
    }
}
