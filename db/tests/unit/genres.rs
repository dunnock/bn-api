use db::dev::TestProject;
use db::models::Genre;

#[test]
fn format_name() {
    assert_eq!(Genre::format_name(&"test".to_string()), "test".to_string());
    assert_eq!(Genre::format_name(&"test ".to_string()), "test".to_string());
    assert_eq!(Genre::format_name(&"Test".to_string()), "test".to_string());
    assert_eq!(Genre::format_name(&"test Genre".to_string()), "test-genre".to_string());
}

#[test]
fn format_names() {
    let names = vec![
        "test".to_string(),
        "test ".to_string(),
        "Test".to_string(),
        "test Genre".to_string(),
    ];
    assert_eq!(
        Genre::format_names(&names),
        vec!["test".to_string(), "test-genre".to_string()]
    );
}

#[test]
fn find_or_create() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let genre_id = Genre::find_or_create(&vec!["test".to_string()], connection).unwrap()[0];

    let genre_ids = Genre::find_or_create(&vec!["test".to_string(), "test2".to_string()], connection).unwrap();
    assert!(genre_ids.contains(&genre_id));

    let genre_ids2 = Genre::find_or_create(&vec!["test".to_string(), "test2".to_string()], connection).unwrap();
    assert_eq!(genre_ids, genre_ids2);
}

#[test]
fn find_by_artist_ids() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();
    let artist2 = project.create_artist().finish();

    // No genres set
    assert!(Genre::find_by_artist_ids(&vec![artist.id, artist2.id], connection)
        .unwrap()
        .is_empty());

    artist
        .set_genres(
            &vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()],
            None,
            connection,
        )
        .unwrap();
    let result = Genre::find_by_artist_ids(&vec![artist.id, artist2.id], connection).unwrap();
    assert_eq!(
        result
            .get(&artist.id)
            .map(|genres| genres.into_iter().map(|g| g.name.clone()).collect::<Vec<String>>()),
        Some(vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()])
    );
    assert!(result.get(&artist2.id).is_none());

    artist2
        .set_genres(&vec!["emo".to_string(), "happy".to_string()], None, connection)
        .unwrap();
    let result = Genre::find_by_artist_ids(&vec![artist.id, artist2.id], connection).unwrap();
    assert_eq!(
        result
            .get(&artist.id)
            .map(|genres| genres.into_iter().map(|g| g.name.clone()).collect::<Vec<String>>()),
        Some(vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()])
    );
    assert_eq!(
        result
            .get(&artist2.id)
            .map(|genres| genres.into_iter().map(|g| g.name.clone()).collect::<Vec<String>>()),
        Some(vec!["emo".to_string(), "happy".to_string()])
    );
}

#[test]
fn all() {
    let project = TestProject::new();
    let connection = project.get_connection();

    // Initial set
    assert_eq!(Genre::all(connection).unwrap().len(), 126);
}
