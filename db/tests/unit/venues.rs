use db::dev::TestProject;
use db::prelude::*;
use db::utils::errors::ErrorCode::ValidationError;

#[test]
fn commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let name = "Name";
    let venue = Venue::create(name.clone(), None, "America/Los_Angeles".into())
        .commit(connection)
        .unwrap();

    assert_eq!(venue.name, name);
    assert_eq!(venue.id.to_string().is_empty(), false);
    assert!(venue.slug_id.is_some());
    let slug = Slug::primary_slug(venue.id, Tables::Venues, connection).unwrap();
    assert_eq!(slug.main_table_id, venue.id);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.slug_type, SlugTypes::Venue);

    // No city slug exists for this record
    assert!(Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::City, connection).is_err());

    // Create venue with default San Francisco, CA, USA city
    let venue = project.create_venue().finish();
    let slug = Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    assert_eq!(slug.main_table_id, venue.id);
    assert_eq!(slug.main_table, Tables::Venues);
    assert_eq!(slug.slug_type, SlugTypes::City);

    // Second venue should also create new slug
    let venue2 = project.create_venue().finish();
    let slug2 = Slug::find_by_type(venue2.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    assert_ne!(&slug, &slug2);
    assert_eq!(&slug.slug, &slug2.slug);
    assert_eq!(slug2.main_table_id, venue2.id);
    assert_eq!(slug2.main_table, Tables::Venues);
    assert_eq!(slug2.slug_type, SlugTypes::City);

    // Different state
    let venue3 = project.create_venue().with_state("MA".to_string()).finish();
    let slug3 = Slug::find_by_type(venue3.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    assert_ne!(&slug, &slug3);
    assert_ne!(&slug.slug, &slug3.slug);
    assert_eq!(slug3.main_table_id, venue3.id);
    assert_eq!(slug3.main_table, Tables::Venues);
    assert_eq!(slug3.slug_type, SlugTypes::City);

    // Matches venue3 so should have same slug as it
    let venue4 = project.create_venue().with_state("MA".to_string()).finish();
    let slug4 = Slug::find_by_type(venue4.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    assert_ne!(&slug3, &slug4);
    assert_eq!(&slug3.slug, &slug4.slug);
    assert_ne!(&slug.slug, &slug4.slug);
    assert_eq!(slug4.main_table_id, venue4.id);
    assert_eq!(slug4.main_table, Tables::Venues);
    assert_eq!(slug4.slug_type, SlugTypes::City);
}

#[test]
fn new_venue_with_validation_errors() {
    let project = TestProject::new();
    let name = "Name";
    let venue = Venue::create(name.clone(), None, "".into());
    let result = venue.commit(project.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("timezone"));
                assert_eq!(errors["timezone"].len(), 1);
                assert_eq!(errors["timezone"][0].code, "length");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update_with_validation_errors() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let parameters = VenueEditableAttributes {
        timezone: Some("".to_string()),
        ..Default::default()
    };

    let result = venue.update(parameters, project.get_connection());
    match result {
        Ok(_) => {
            panic!("Expected validation error");
        }
        Err(error) => match &error.error_code {
            ValidationError { errors } => {
                assert!(errors.contains_key("timezone"));
                assert_eq!(errors["timezone"].len(), 1);
                assert_eq!(errors["timezone"][0].code, "length");
            }
            _ => panic!("Expected validation error"),
        },
    }
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();

    let slug = Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    let new_name = "New Venue Name";
    let new_address = "Test Address";
    let parameters = VenueEditableAttributes {
        name: Some(new_name.to_string()),
        address: Some(new_address.to_string()),
        ..Default::default()
    };

    let venue = venue.update(parameters, project.get_connection()).unwrap();
    let found_slug = Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::City, connection).unwrap();
    assert_eq!(slug.slug, found_slug.slug);
    assert_eq!(slug.main_table, found_slug.main_table);
    assert_eq!(slug.main_table_id, found_slug.main_table_id);
    assert_eq!(venue.name, new_name);
    assert_eq!(venue.address, new_address);

    let new_city = "Test City";
    let parameters = VenueEditableAttributes {
        city: Some(new_city.to_string()),
        ..Default::default()
    };
    let venue = venue.update(parameters, project.get_connection()).unwrap();
    assert_eq!(venue.city, new_city);
    let slug2 = Slug::find_by_type(venue.id, Tables::Venues, SlugTypes::City, connection).unwrap();

    // Slug has changed with new city name and old slug has been removed
    assert_ne!(slug.id, slug2.id);
    assert_ne!(slug.slug, slug2.slug);
    assert!(Slug::find(slug.id, connection).is_err());
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let slug = Slug::primary_slug(venue.id, Tables::Venues, connection).unwrap();
    let display_venue = venue.for_display(connection).unwrap();

    let city_slug = Slug::find_first_for_city(&venue.city, &venue.state, &venue.country, connection).unwrap();
    assert_eq!(display_venue.id, venue.id);
    assert_eq!(display_venue.slug, slug.slug);
    assert_eq!(display_venue.city_slug, Some(city_slug.slug));

    // No city so no slug
    let parameters = VenueEditableAttributes {
        city: Some("".to_string()),
        ..Default::default()
    };
    let venue = venue.update(parameters, project.get_connection()).unwrap();
    let display_venue = venue.for_display(connection).unwrap();
    assert_eq!(display_venue.id, venue.id);
    assert_eq!(display_venue.slug, slug.slug);
    assert_eq!(display_venue.city_slug, None);
}

#[test]
fn set_privacy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut venue = project.create_venue().finish();
    assert!(!venue.is_private);

    venue = venue.set_privacy(true, connection).unwrap();
    assert!(venue.is_private);

    venue = venue.set_privacy(false, connection).unwrap();
    assert!(!venue.is_private);
}

#[test]
fn find() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let found_venue = Venue::find(venue.id, project.get_connection()).unwrap();
    assert_eq!(venue, found_venue);
}

#[test]
fn find_by_ids() {
    let project = TestProject::new();
    let venue = project.create_venue().with_name("Venue1".to_string()).finish();
    let _ = project.create_venue().with_name("Venue2".to_string()).finish();
    let venue3 = project.create_venue().with_name("Venue3".to_string()).finish();

    let mut expected_venues = vec![venue.clone(), venue3.clone()];
    expected_venues.sort_by_key(|v| v.id);

    let found_venues = Venue::find_by_ids(vec![venue.id, venue3.id], &project.get_connection()).unwrap();
    assert_eq!(expected_venues, found_venues);
}

#[test]
fn all() {
    let project = TestProject::new();
    let venue = project.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = project.create_venue().with_name("Venue2".to_string()).finish();
    let organization = project.create_organization().finish();
    let conn = &project.get_connection();

    let all_found_venues = Venue::all(None, conn).unwrap();
    let mut all_venues = vec![venue, venue2];
    assert_eq!(all_venues, all_found_venues);

    let venue3 = project
        .create_venue()
        .with_name("Venue3".to_string())
        .make_private()
        .finish();
    venue3.add_to_organization(organization.id, conn).unwrap();
    let user = project.create_user().finish();
    let _org_user = organization
        .add_user(user.id, vec![Roles::OrgMember], Vec::new(), conn)
        .unwrap();
    all_venues.push(venue3);
    let all_found_venues = Venue::all(Some(&user), conn).unwrap();
    assert_eq!(all_venues, all_found_venues);
    let all_found_venues = Venue::all(Some(&User::find(user.id, conn).unwrap()), conn).unwrap();
    assert_eq!(all_venues, all_found_venues);
}

#[test]
fn find_for_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let owner = project.create_user().finish();
    let member = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project
        .create_user()
        .finish()
        .add_role(Roles::Admin, connection)
        .unwrap();
    let _public_venue = project.create_venue().with_name("Venue0".to_string()).finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();
    let venue1 = project
        .create_venue()
        .with_name("Venue1".to_string())
        .with_organization(&organization)
        .finish();

    let venue2 = project
        .create_venue()
        .with_name("Venue2".to_string())
        .with_organization(&organization)
        .finish();

    let venue3 = project
        .create_venue()
        .with_name("Venue3".to_string())
        .with_organization(&organization)
        .make_private()
        .finish();

    // Add another venue for another org to make sure it isn't included
    let organization2 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _venue4 = project
        .create_venue()
        .with_name("Venue4".to_string())
        .with_organization(&organization2)
        .finish();

    let user = project.create_user().finish();

    let public_organization_venues = vec![venue1.clone(), venue2.clone()];
    let organization_venues = vec![venue1, venue2, venue3];

    // Guest user / not logged in
    let found_venues = Venue::find_for_organization(None, organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, public_organization_venues);

    // Private venue is not shown for users
    let found_venues = Venue::find_for_organization(Some(&user), organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, public_organization_venues);

    // Private venue is shown for admins, owners, and members
    let found_venues = Venue::find_for_organization(Some(&owner), organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, organization_venues);

    let found_venues = Venue::find_for_organization(Some(&member), organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, organization_venues);

    let found_venues = Venue::find_for_organization(Some(&admin), organization.id, project.get_connection()).unwrap();
    assert_eq!(found_venues, organization_venues);
}

#[test]
fn organizations() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let venue = project.create_venue().with_organization(&organization).finish();
    let venue2 = project.create_venue().finish();

    assert_eq!(Ok(vec![organization]), venue.organizations(connection));
    assert_eq!(Ok(Vec::new()), venue2.organizations(connection));
}
