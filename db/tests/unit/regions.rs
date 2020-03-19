use db::dev::TestProject;
use db::models::{Region, RegionEditableAttributes};
use uuid::Uuid;

#[test]
fn create() {
    let project = TestProject::new();
    let name = "Name";
    let region = Region::create(name.into()).commit(project.get_connection()).unwrap();

    assert_eq!(region.name, name);
    assert_eq!(region.id.to_string().is_empty(), false);
}

#[test]
fn update() {
    let project = TestProject::new();
    let region = project.create_region().finish();

    let new_name = "New Region Name";

    let parameters = RegionEditableAttributes {
        name: Some(new_name.to_string()),
    };

    let updated_region = region.update(parameters, project.get_connection()).unwrap();
    assert_eq!(updated_region.name, new_name);
}

#[test]
fn find() {
    let project = TestProject::new();
    let region = project.create_region().finish();

    let found_region = Region::find(region.id, project.get_connection()).unwrap();
    assert_eq!(region, found_region);

    let found_by = Region::find_by_name(&region.name, project.get_connection())
        .unwrap()
        .unwrap();
    assert_eq!(region, found_by);
}

#[test]
fn all() {
    let project = TestProject::new();
    let region = project.create_region().with_name("Region1".into()).finish();
    let region2 = project.create_region().with_name("Region2".into()).finish();
    let other_region = Region::find(Uuid::nil(), project.get_connection()).unwrap();

    let all_found_regions = Region::all(project.get_connection()).unwrap();
    let all_regions = vec![other_region, region, region2];
    assert_eq!(all_regions, all_found_regions);
}
