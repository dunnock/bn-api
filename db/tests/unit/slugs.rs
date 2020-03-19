use db::dev::TestProject;
use db::models::*;
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let slug_text = "slug-123".to_string();
    let main_table = Tables::Venues;
    let main_table_id = Uuid::new_v4();
    let slug_type = SlugTypes::Venue;

    let slug = Slug::create(slug_text.clone(), main_table, main_table_id, slug_type, None, None)
        .commit(connection)
        .unwrap();

    assert_eq!(slug.slug, slug_text);
    assert_eq!(slug.main_table, main_table);
    assert_eq!(slug.main_table_id, main_table_id);
    assert_eq!(slug.slug_type, slug_type);
}

#[test]
fn update_slug() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let slug_text = "slug-123".to_string();
    let main_table = Tables::Venues;
    let main_table_id = Uuid::new_v4();
    let slug_type = SlugTypes::Venue;

    let slug = Slug::create(slug_text.clone(), main_table, main_table_id, slug_type, None, None)
        .commit(connection)
        .unwrap();

    assert_eq!(slug.slug, slug_text);
    assert_eq!(slug.main_table, main_table);
    assert_eq!(slug.main_table_id, main_table_id);
    assert_eq!(slug.slug_type, slug_type);
    let attributes = SlugEditableAttributes {
        title: Some(Some("New Title".to_string())),
        description: Some(Some("New Description".to_string())),
    };
    let updated_slug = slug.update(attributes, connection).unwrap();
    assert_eq!(updated_slug.slug, slug_text);
    assert_eq!(updated_slug.main_table, main_table);
    assert_eq!(updated_slug.main_table_id, main_table_id);
    assert_eq!(updated_slug.slug_type, slug_type);
    assert_eq!(updated_slug.title, Some("New Title".to_string()));
    assert_eq!(updated_slug.description, Some("New Description".to_string()));
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let slug = project.create_slug().finish();

    let found_slug = Slug::find(slug.id, connection).unwrap();
    assert_eq!(slug, found_slug);
}

#[test]
fn search() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let slug_venue = project
        .create_slug()
        .with_slug("custom-slug-venue")
        .with_type(SlugTypes::Venue)
        .finish();
    let found_slugs = Slug::search(Some("custom".to_string()), None, 0, 10, connection).unwrap();
    assert_eq!(slug_venue, found_slugs.0[0]);
    let slug_city = project
        .create_slug()
        .with_slug("custom-slug-city")
        .with_type(SlugTypes::City)
        .finish();
    let found_slugs = Slug::search(Some("custom".to_string()), None, 0, 10, connection).unwrap();
    assert_eq!(found_slugs.1, 2);
    let found_slugs = Slug::search(Some("custom".to_string()), Some(SlugTypes::City), 0, 10, connection).unwrap();
    assert_eq!(slug_city, found_slugs.0[0]);
}

#[test]
fn find_by_slug() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let slug = project.create_slug().finish();
    let found_slugs = Slug::find_by_slug(&slug.slug, connection).unwrap();
    assert_eq!(vec![slug.clone()], found_slugs);

    // Slug matches, is included
    let slug2 = project.create_slug().with_slug(&slug.slug).finish();
    let mut found_slugs = Slug::find_by_slug(&slug.slug, connection).unwrap();
    found_slugs.sort_by_key(|s| s.id);
    let mut expected_slugs = vec![slug.clone(), slug2];
    expected_slugs.sort_by_key(|s| s.id);
    assert_eq!(&expected_slugs, &found_slugs);

    // New slug, not included
    let slug3 = project.create_slug().finish();
    let mut found_slugs = Slug::find_by_slug(&slug.slug, connection).unwrap();
    found_slugs.sort_by_key(|s| s.id);
    assert_eq!(&expected_slugs, &found_slugs);

    // New slug, is included with its own slug
    let found_slugs = Slug::find_by_slug(&slug3.slug, connection).unwrap();
    assert_eq!(vec![slug3.clone()], found_slugs);
}

#[test]
fn automatic_genre_slug_creation() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let new_genres = vec!["custom-1", "custom-2"];
    project.create_genre().with_name(&new_genres[0].to_string()).finish();

    let genre_slugs = Slug::find_by_slug_type(SlugTypes::Genre, connection).unwrap();
    let genre_slugs = genre_slugs
        .into_iter()
        .filter(|i| i.slug.as_str().starts_with("custom-"))
        .collect::<Vec<Slug>>();

    assert_eq!(genre_slugs.len(), 1);
    assert_eq!(genre_slugs[0].slug, "custom-1".to_string());

    project.create_genre().with_name(&new_genres[1].to_string()).finish();
    let genre_slugs = Slug::find_by_slug_type(SlugTypes::Genre, connection).unwrap();
    let genre_slugs = genre_slugs
        .into_iter()
        .filter(|i| i.slug.as_str().starts_with("custom-"))
        .collect::<Vec<Slug>>();
    assert_eq!(genre_slugs.len(), 2);
}

#[test]
fn find_by_type() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let venue = project.create_venue().finish();
    let organization = project.create_organization().finish();

    let slug = Slug::primary_slug(event.id, Tables::Events, connection).unwrap();
    let slug2 = Slug::primary_slug(venue.id, Tables::Venues, connection).unwrap();
    let slug3 = Slug::primary_slug(organization.id, Tables::Organizations, connection).unwrap();

    let found_slug = Slug::find_by_type(event.id, Tables::Events, SlugTypes::Event, connection).unwrap();
    assert_eq!(slug, found_slug);

    let found_slug = Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::Venue, connection).unwrap();
    assert_eq!(slug2, found_slug);

    let found_slug = Slug::find_by_type(
        organization.id,
        Tables::Organizations,
        SlugTypes::Organization,
        connection,
    )
    .unwrap();
    assert_eq!(slug3, found_slug);

    // Unable to find slug given wrong table
    assert!(Slug::find_by_type(event.id, Tables::Venues, SlugTypes::Venue, connection).is_err());
    assert!(Slug::find_by_type(event.id, Tables::Organizations, SlugTypes::Organization, connection).is_err());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let slug = project.create_slug().for_event(&event).finish();

    assert!(Slug::find(slug.id, connection).is_ok());
    Slug::destroy(event.id, Tables::Events, SlugTypes::Event, connection).unwrap();
    assert!(Slug::find(slug.id, connection).is_err());
}

#[test]
fn primary_slugs() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event = project.create_event().finish();
    let event2 = project.create_event().finish();
    let organization = project.create_organization().finish();
    let venue = project.create_venue().finish();

    // Multiple slugs returned
    let mut slugs: Vec<Uuid> = Slug::load_primary_slugs(&vec![event.id, event2.id], Tables::Events, connection)
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    slugs.sort();
    let mut expected_slugs = vec![event.slug_id.unwrap(), event2.slug_id.unwrap()];
    expected_slugs.sort();
    assert_eq!(slugs, expected_slugs);

    // Individual slugs
    let slug = Slug::primary_slug(event.id, Tables::Events, connection).unwrap();
    assert_eq!(Some(slug.id), event.slug_id);

    let slug = Slug::primary_slug(event2.id, Tables::Events, connection).unwrap();
    assert_eq!(Some(slug.id), event2.slug_id);

    let slug = Slug::primary_slug(venue.id, Tables::Venues, connection).unwrap();
    assert_eq!(Some(slug.id), venue.slug_id);

    let slug = Slug::primary_slug(organization.id, Tables::Organizations, connection).unwrap();
    assert_eq!(Some(slug.id), organization.slug_id);
}

#[test]
fn find_first_for_city() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // US city
    let city = "Oakland".to_string();
    let state = "California".to_string();
    let country = "US".to_string();
    // No records exist for this slug
    assert!(Slug::find_first_for_city(&city, &state, &country, connection).is_err());
    let venue = project
        .create_venue()
        .with_city(city.clone())
        .with_state(state.clone())
        .with_country(country.clone())
        .finish();
    let slug = Slug::find_first_for_city(&city, &state, &country, connection).unwrap();
    assert_eq!(slug.main_table_id, venue.id);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.slug_type, SlugTypes::City);
    assert_eq!(slug.slug, "oakland".to_string());

    // Country without states
    let city = "Cape Town".to_string();
    let state = "".to_string();
    let country = "SA".to_string();
    // No records exist for this slug
    assert!(Slug::find_first_for_city(&city, &state, &country, connection).is_err());
    let venue = project
        .create_venue()
        .with_city(city.clone())
        .with_state(state.clone())
        .with_country(country.clone())
        .finish();
    let slug = Slug::find_first_for_city(&city, &state, &country, connection).unwrap();
    assert_eq!(slug.main_table_id, venue.id);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.slug_type, SlugTypes::City);
    assert_eq!(slug.slug, "cape-town".to_string());
}

#[test]
fn generate_slug() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let event = project.create_event().finish();
    let venue = project
        .create_venue()
        .with_city("Oakland".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();
    let event2 = project.create_event().with_venue(&venue).finish();
    let venue2 = project
        .create_venue()
        .with_city("Oakland".to_string())
        .with_state("California".to_string())
        .with_country("US".to_string())
        .finish();
    let venue3 = project
        .create_venue()
        .with_city("Oakland".to_string())
        .with_state("California".to_string())
        .with_country("NL".to_string())
        .finish();
    let venue4 = project
        .create_venue()
        .with_city("Oakland".to_string())
        .with_state("California".to_string())
        .with_country("NL".to_string())
        .finish();
    let organization_slug_context = SlugContext::Organization {
        id: organization.id,
        name: "Zephyr".to_string(),
    };
    let event_no_venue_slug_context = SlugContext::Event {
        id: event.id,
        name: "event".to_string(),
        venue: None,
    };
    let event_with_venue_slug_context = SlugContext::Event {
        id: event2.id,
        name: "event2".to_string(),
        venue: Some(venue.clone()),
    };
    let venue_slug_context = SlugContext::Venue {
        id: venue.id,
        name: "venue".to_string(),
        city: venue.city.clone(),
        state: venue.state.clone(),
        country: venue.country.clone(),
    };
    let duplicate_venue_slug_context = SlugContext::Venue {
        id: venue2.id,
        name: "venue".to_string(),
        city: venue2.city.clone(),
        state: venue2.state.clone(),
        country: venue2.country.clone(),
    };
    let unique_country_venue_slug_context = SlugContext::Venue {
        id: venue3.id,
        name: "venue".to_string(),
        city: venue3.city.clone(),
        state: venue3.state.clone(),
        country: venue3.country.clone(),
    };
    let duplicate_unique_country_venue_slug_context = SlugContext::Venue {
        id: venue4.id,
        name: "venue".to_string(),
        city: venue4.city.clone(),
        state: venue4.state.clone(),
        country: venue4.country.clone(),
    };

    // Generate organization slug
    let slug = Slug::generate_slug(&organization_slug_context, SlugTypes::Organization, connection).unwrap();
    assert_eq!(&slug.slug, "zephyr");
    assert_eq!(slug.slug_type, SlugTypes::Organization);
    assert_eq!(slug.main_table, Tables::Organizations);
    assert_eq!(slug.main_table_id, organization.id);

    // Generate event with no venue slug
    let slug = Slug::generate_slug(&event_no_venue_slug_context, SlugTypes::Event, connection).unwrap();
    assert_eq!(&slug.slug, "event");
    assert_eq!(slug.slug_type, SlugTypes::Event);
    assert_eq!(slug.main_table, Tables::Events);
    assert_eq!(slug.main_table_id, event.id);

    // Generate event with venue
    let slug = Slug::generate_slug(&event_with_venue_slug_context, SlugTypes::Event, connection).unwrap();
    assert_eq!(&slug.slug, "event2-oakland");
    assert_eq!(slug.slug_type, SlugTypes::Event);
    assert_eq!(slug.main_table, Tables::Events);
    assert_eq!(slug.main_table_id, event2.id);

    // Generate venue slug
    let slug = Slug::generate_slug(&venue_slug_context, SlugTypes::Venue, connection).unwrap();
    assert_eq!(&slug.slug, "venue");
    assert_eq!(slug.slug_type, SlugTypes::Venue);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.main_table_id, venue.id);

    // Generate duplicate venue slug
    let duplicate_slug = Slug::generate_slug(&duplicate_venue_slug_context, SlugTypes::Venue, connection).unwrap();
    assert!(&duplicate_slug.slug.starts_with("venue-"));
    assert_ne!(slug.id, duplicate_slug.id);
    assert_eq!(duplicate_slug.slug_type, SlugTypes::Venue);
    assert_eq!(duplicate_slug.main_table, Tables::Venues);
    assert_eq!(duplicate_slug.main_table_id, venue2.id);

    // Generate city slug
    let slug = Slug::generate_slug(&venue_slug_context, SlugTypes::City, connection).unwrap();
    assert_eq!(&slug.slug, "oakland");
    assert_eq!(slug.slug_type, SlugTypes::City);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.main_table_id, venue.id);

    // Generate duplicate city slug (but with same address)
    let duplicate_slug = Slug::generate_slug(&duplicate_venue_slug_context, SlugTypes::City, connection).unwrap();
    assert_eq!(&duplicate_slug.slug, "oakland");
    assert_ne!(slug.id, duplicate_slug.id);
    assert_eq!(duplicate_slug.slug_type, SlugTypes::City);
    assert_eq!(duplicate_slug.main_table, Tables::Venues);
    assert_eq!(duplicate_slug.main_table_id, venue2.id);

    // Generate unique city slug via new country
    let unique_slug = Slug::generate_slug(&unique_country_venue_slug_context, SlugTypes::City, connection).unwrap();
    assert!(&unique_slug.slug.starts_with("oakland-"));
    assert_ne!(slug.id, unique_slug.id);
    assert_eq!(unique_slug.slug_type, SlugTypes::City);
    assert_eq!(unique_slug.main_table, Tables::Venues);
    assert_eq!(unique_slug.main_table_id, venue3.id);

    // Generate unique city slug via new country
    let duplicate_unique_slug = Slug::generate_slug(
        &duplicate_unique_country_venue_slug_context,
        SlugTypes::City,
        connection,
    )
    .unwrap();
    assert_eq!(&unique_slug.slug, &duplicate_unique_slug.slug);
    assert_ne!(duplicate_unique_slug.id, unique_slug.id);
    assert_eq!(duplicate_unique_slug.slug_type, SlugTypes::City);
    assert_eq!(duplicate_unique_slug.main_table, Tables::Venues);
    assert_eq!(duplicate_unique_slug.main_table_id, venue4.id);
}
