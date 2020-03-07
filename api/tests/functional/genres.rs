use bigneon_api::controllers::genres;
use bigneon_api::controllers::genres::GenreListItem;
use bigneon_db::models::Genre;
use serde_json;
use crate::support;
use crate::support::database::TestDatabase;

#[test]
fn index() {
    let database = TestDatabase::new();
    let connection = database.connection.get();
    let expected_genres: Vec<GenreListItem> = Genre::all(connection)
        .unwrap()
        .iter()
        .map(|g| GenreListItem {
            id: g.id,
            name: g.name.clone(),
        })
        .collect();
    let response = genres::index(database.connection.clone().into()).unwrap();
    let expected_genres_json = serde_json::to_string(&json!({ "genres": expected_genres })).unwrap();

    let body = support::unwrap_body_to_string(&response).unwrap();
    assert_eq!(body, expected_genres_json);
}
