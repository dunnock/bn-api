use db::dev::TestProject;
use db::prelude::*;
use db::utils::dates;

#[test]
fn new_organization_interaction_commit() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let user = project.create_user().finish();
    let first_interaction_date = dates::now().add_minutes(-1).finish();
    let last_interaction_date = dates::now().finish();
    let interaction_count = 1;

    let organization_interaction = OrganizationInteraction::create(
        organization.id,
        user.id,
        first_interaction_date,
        last_interaction_date,
        interaction_count,
    )
    .commit(connection)
    .unwrap();

    assert_eq!(interaction_count, organization_interaction.interaction_count);
    assert_eq!(
        first_interaction_date.timestamp_subsec_millis(),
        organization_interaction.first_interaction.timestamp_subsec_millis()
    );
    assert_eq!(
        last_interaction_date.timestamp_subsec_millis(),
        organization_interaction.last_interaction.timestamp_subsec_millis()
    );
}

#[test]
fn find_by_organization_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let user = project.create_user().finish();
    let first_interaction_date = dates::now().add_minutes(-1).finish();
    let last_interaction_date = dates::now().finish();
    let interaction_count = 1;

    let organization_interaction = OrganizationInteraction::create(
        organization.id,
        user.id,
        first_interaction_date,
        last_interaction_date,
        interaction_count,
    )
    .commit(connection)
    .unwrap();

    assert_eq!(
        organization_interaction,
        OrganizationInteraction::find_by_organization_user(organization.id, user.id, connection).unwrap()
    );
}

#[test]
fn update() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let organization = project.create_organization().finish();
    let user = project.create_user().finish();
    let first_interaction_date = dates::now().add_minutes(-1).finish();
    let last_interaction_date = dates::now().finish();
    let interaction_count = 1;

    let organization_interaction = OrganizationInteraction::create(
        organization.id,
        user.id,
        first_interaction_date,
        last_interaction_date,
        interaction_count,
    )
    .commit(connection)
    .unwrap();

    assert_eq!(interaction_count, organization_interaction.interaction_count);
    assert_eq!(
        first_interaction_date.timestamp_subsec_millis(),
        organization_interaction.first_interaction.timestamp_subsec_millis()
    );
    assert_eq!(
        last_interaction_date.timestamp_subsec_millis(),
        organization_interaction.last_interaction.timestamp_subsec_millis()
    );

    let new_first_interaction_date = dates::now().add_minutes(1).finish();
    let new_last_interaction_date = dates::now().add_minutes(2).finish();
    let new_interaction_count = 2;
    let organization_interaction = organization_interaction
        .update(
            &OrganizationInteractionEditableAttributes {
                interaction_count: Some(new_interaction_count),
                first_interaction: Some(new_first_interaction_date),
                last_interaction: Some(new_last_interaction_date),
            },
            connection,
        )
        .unwrap();

    assert_eq!(new_interaction_count, organization_interaction.interaction_count);
    assert_eq!(
        new_first_interaction_date.timestamp_subsec_millis(),
        organization_interaction.first_interaction.timestamp_subsec_millis()
    );
    assert_eq!(
        new_last_interaction_date.timestamp_subsec_millis(),
        organization_interaction.last_interaction.timestamp_subsec_millis()
    );
}
