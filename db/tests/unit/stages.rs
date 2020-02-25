use bigneon_db::dev::TestProject;
use bigneon_db::prelude::*;

#[test]
fn commit() {
    let project = TestProject::new();
    let name = "Name";
    let connection = project.get_connection();
    let venue = Venue::create(name.clone(), None, "America/Los_Angeles".into())
        .commit(connection)
        .unwrap();
    let stage_name = "Stage Name".to_string();
    let stage = Stage::create(
        venue.id,
        stage_name.clone(),
        Some("Description".to_string()),
        Some(1000),
    )
    .commit(connection)
    .unwrap();

    assert_eq!(stage.name, stage_name);
    assert_eq!(stage.id.to_string().is_empty(), false);
}

#[test]
fn update() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let connection = project.get_connection();
    let stage = Stage::create(venue.id, "Stage Name".to_string(), None, None)
        .commit(connection)
        .unwrap();
    let new_name = "New Stage Name".to_string();
    let new_capacity = 1000;

    let parameters = StageEditableAttributes {
        name: Some(new_name.clone()),
        capacity: Some(Some(new_capacity)),
        ..Default::default()
    };

    let update_stage = stage.update(parameters, connection).unwrap();
    assert_eq!(update_stage.name, new_name);
    assert_eq!(update_stage.capacity, Some(new_capacity));
    assert_eq!(update_stage.description, None);
}

#[test]
fn find() {
    let project = TestProject::new();
    let venue = project.create_venue().finish();
    let stage = project.create_stage().with_venue_id(venue.id).finish();

    let found_stage = Stage::find(stage.id, project.get_connection()).unwrap();
    assert_eq!(stage, found_stage);
}

#[test]
fn all_for_venue() {
    let project = TestProject::new();
    let venue = project.create_venue().with_name("Venue 1".to_string()).finish();

    let venue_2 = project.create_venue().with_name("Venue 2".to_string()).finish();

    let stage_1 = project
        .create_stage()
        .with_name("Stage 1".to_string())
        .with_venue_id(venue.id.clone())
        .finish();
    let stage_2 = project
        .create_stage()
        .with_name("Stage 2".to_string())
        .with_venue_id(venue.id.clone())
        .finish();
    let stage_3 = project
        .create_stage()
        .with_name("Stage 3".to_string())
        .with_venue_id(venue.id.clone())
        .finish();
    let _stage_4 = project.create_stage().with_venue_id(venue_2.id.clone()).finish();

    let conn = project.get_connection();
    let venue_1_stages = Stage::find_by_venue_id(venue.id.clone(), conn).unwrap();

    let all_stages = vec![stage_1, stage_2, stage_3];
    assert_eq!(venue_1_stages, all_stages);
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let venue = project.create_venue().with_name("Venue 1".to_string()).finish();

    let stage_1 = project
        .create_stage()
        .with_name("Stage 1".to_string())
        .with_venue_id(venue.id.clone())
        .finish();
    let stage_2 = project
        .create_stage()
        .with_name("Stage 2".to_string())
        .with_venue_id(venue.id.clone())
        .finish();
    let stage_3 = project
        .create_stage()
        .with_name("Stage 3".to_string())
        .with_venue_id(venue.id.clone())
        .finish();

    let conn = project.get_connection();
    stage_3.destroy(conn).unwrap();
    let venue_1_stages = Stage::find_by_venue_id(venue.id.clone(), conn).unwrap();

    let all_stages = vec![stage_1, stage_2];
    assert_eq!(venue_1_stages, all_stages);
}
