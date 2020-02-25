use bigneon_db::dev::TestProject;
use bigneon_db::models::OrganizationVenue;
use bigneon_db::utils::errors::DatabaseError;

#[test]
fn create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let organization = project.create_organization().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();

    assert_eq!(organization_venue.id.to_string().is_empty(), false);
    assert_eq!(organization_venue.venue_id, venue.id);
    assert_eq!(organization_venue.organization_id, organization.id);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let organization = project.create_organization().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(connection)
        .unwrap();

    assert_eq!(
        organization_venue.destroy(connection),
        DatabaseError::business_process_error(
            "Unable to remove organization venue link, at least one organization must be associated with venue",
        )
    );

    // Add second organization
    let organization2 = project.create_organization().finish();
    let organization_venue2 = OrganizationVenue::create(organization2.id, venue.id)
        .commit(connection)
        .unwrap();
    // Can delete the other organization now
    assert!(organization_venue.destroy(connection).is_ok(),);

    // Now we're back to one so can't delete the other organization venue
    assert_eq!(
        organization_venue2.destroy(connection),
        DatabaseError::business_process_error(
            "Unable to remove organization venue link, at least one organization must be associated with venue",
        )
    );
}

#[test]
fn find_by_venue() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let organization = project
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(project.get_connection())
        .unwrap();
    let organization_venue2 = OrganizationVenue::create(organization2.id, venue.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        OrganizationVenue::find_by_venue(venue.id, None, None, connection)
            .unwrap()
            .data,
        vec![organization_venue, organization_venue2]
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let organization = project.create_organization().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(project.get_connection())
        .unwrap();
    assert_eq!(
        OrganizationVenue::find(organization_venue.id, connection).unwrap(),
        organization_venue
    );
}

#[test]
fn find_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = project.create_venue().with_name("Venue2".to_string()).finish();
    let organization = project.create_organization().finish();
    let organization_venue = OrganizationVenue::create(organization.id, venue.id)
        .commit(project.get_connection())
        .unwrap();
    let organization_venue2 = OrganizationVenue::create(organization.id, venue2.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        OrganizationVenue::find_by_organization(organization.id, None, None, connection)
            .unwrap()
            .data,
        vec![organization_venue, organization_venue2]
    );
}

#[test]
fn find_organizations_by_venue() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().finish();
    let organization = project
        .create_organization()
        .with_name("Organization1".to_string())
        .finish();
    let organization2 = project
        .create_organization()
        .with_name("Organization2".to_string())
        .finish();
    OrganizationVenue::create(organization.id, venue.id)
        .commit(project.get_connection())
        .unwrap();
    OrganizationVenue::create(organization2.id, venue.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        OrganizationVenue::find_organizations_by_venue(venue.id, connection).unwrap(),
        vec![organization, organization2]
    );
}

#[test]
fn find_venues_by_organization() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let venue = project.create_venue().with_name("Venue1".to_string()).finish();
    let venue2 = project.create_venue().with_name("Venue2".to_string()).finish();
    let organization = project.create_organization().finish();
    OrganizationVenue::create(organization.id, venue.id)
        .commit(project.get_connection())
        .unwrap();
    OrganizationVenue::create(organization.id, venue2.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        OrganizationVenue::find_venues_by_organization(organization.id, connection).unwrap(),
        vec![venue, venue2]
    );
}
