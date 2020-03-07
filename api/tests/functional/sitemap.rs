use bigneon_api::utils::gen_sitemap;
use bigneon_db::prelude::*;
use crate::support::database::TestDatabase;

#[test]
fn index() {
    let database = TestDatabase::new();
    let conn = database.connection.get();
    let organization = database.create_organization().finish();

    let venue = database
        .create_venue()
        .with_city("San Francisco".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();

    let event = database
        .create_event()
        .with_name("NewEvent1".to_string())
        .with_organization(&organization)
        .with_venue(&venue)
        .finish();

    database
        .create_slug()
        .for_event(&event)
        .with_slug("redirect-me")
        .finish();

    let front_end_url = "http://localhost:3000".to_string();

    //find all active events
    let events_slug_ids = Event::find_all_active_events(conn)
        .unwrap()
        .iter()
        .map(|e| e.slug_id)
        .filter(|s| s.is_some())
        .map(|s| s.unwrap())
        .collect();

    let event_urls = gen_sitemap::create_urls(&front_end_url, events_slug_ids, "tickets".to_string(), conn);

    // Cities
    let city_slug_id = Slug::find_by_slug_type(SlugTypes::City, conn)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    let city_urls = gen_sitemap::create_urls(&front_end_url, city_slug_id, "cities".to_string(), conn);

    // Venues
    let venue_slugs_ids = Slug::find_by_slug_type(SlugTypes::Venue, conn)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    let venue_urls = gen_sitemap::create_urls(&front_end_url, venue_slugs_ids, "venues".to_string(), conn);

    // Organizations
    let organizations_slugs_ids = Slug::find_by_slug_type(SlugTypes::Organization, conn)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    let organizations_urls = gen_sitemap::create_urls(
        &front_end_url,
        organizations_slugs_ids,
        "organizations".to_string(),
        conn,
    );

    let mut urls = event_urls;
    urls.extend(venue_urls);
    urls.extend(organizations_urls);
    urls.extend(city_urls);

    let sitemap_xml_test = gen_sitemap::create_sitemap(&urls).unwrap();
    let sitemap_xml = gen_sitemap::create_sitemap_conn(conn, &front_end_url).unwrap();
    assert_eq!(sitemap_xml_test, sitemap_xml);
}
