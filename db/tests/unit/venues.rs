use bigneon_db::models::{Venue, VenueEditableAttributes};
use support::project::TestProject;

#[test]
fn commit() {
    let project = TestProject::new();
    let name = "Name";
    let venue = Venue::create(name.clone()).commit(&project).unwrap();

    assert_eq!(venue.name, name);
    assert_eq!(venue.id.to_string().is_empty(), false);
}

#[test]
fn update() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let new_name = "New Venue Name";
    let new_address = "Test Address";

    let parameters = VenueEditableAttributes {
        name: Some(new_name.to_string()),
        address: Some(new_address.to_string()),
        city: None,
        state: None,
        country: None,
        zip: None,
        phone: None,
    };

    let updated_venue = venue.update(parameters, &project).unwrap();
    assert_eq!(updated_venue.name, new_name);
    assert_eq!(updated_venue.address.unwrap(), new_address);
}

#[test]
fn find() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();

    let found_venue = Venue::find(&venue.id, &project).unwrap();
    assert_eq!(venue, found_venue);

    let venue2 = project.create_venue().finish();

    let all_found_venues = Venue::all(&project).unwrap();
    let all_venues = vec![venue, venue2];
    assert_eq!(all_venues, all_found_venues);
}

#[test]
fn has_organization() {
    let mut project = TestProject::new();
    let venue = project.create_venue().finish();
    let user = project.create_user().finish();
    let organization = project.create_organization().with_owner(&user).finish();
    let organization2 = project.create_organization().with_owner(&user).finish();
    assert!(!venue.has_organization(organization.id, &project).unwrap());
    assert!(!venue.has_organization(organization2.id, &project).unwrap());

    venue
        .add_to_organization(&organization.id, &project)
        .unwrap();

    assert!(venue.has_organization(organization.id, &project).unwrap());
    assert!(!venue.has_organization(organization2.id, &project).unwrap());
}

#[test]
fn create_find_via_org() {
    let mut project = TestProject::new();

    let venue = project.create_venue().finish();
    let venue2 = project.create_venue().finish();

    let user = project.create_user().finish();
    let organization = project.create_organization().with_owner(&user).finish();

    venue
        .add_to_organization(&organization.id, &project)
        .unwrap();
    venue2
        .add_to_organization(&organization.id, &project)
        .unwrap();

    let all_venues = vec![venue, venue2];

    let found_venues = organization.venues(&project).unwrap();
    assert_eq!(found_venues, all_venues);
    let found_venues = Venue::find_for_organization(organization.id, &project).unwrap();
    assert_eq!(found_venues, all_venues);

    // Add another venue for another org to make sure it isn't included
    let other_venue = project.create_venue().finish();
    let organization2 = project.create_organization().with_owner(&user).finish();
    other_venue
        .add_to_organization(&organization2.id, &project)
        .unwrap();

    let found_venues = Venue::find_for_organization(organization.id, &project).unwrap();
    assert_eq!(found_venues, all_venues);
    assert!(!found_venues.contains(&other_venue));
}