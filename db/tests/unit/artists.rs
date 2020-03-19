use db::dev::TestProject;
use db::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

#[test]
fn find_spotify_linked_artists() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project
        .create_artist()
        .with_name("Artist 1".to_string())
        .with_spotify_id("spotify_1".to_string())
        .finish();
    let artist2 = project
        .create_artist()
        .with_name("Artist 2".to_string())
        .with_spotify_id("spotify_2".to_string())
        .finish();
    let _artist3 = project.create_artist().with_name("Artist 3".to_string()).finish();

    assert_eq!(
        Artist::find_spotify_linked_artists(connection),
        Ok(vec![artist, artist2])
    );
}

#[test]
fn set_genres() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let creator = project.create_user().finish();
    let artist = project.create_artist().with_name("Artist 1".to_string()).finish();
    let artist2 = project.create_artist().with_name("Artist 2".to_string()).finish();

    // No genres set
    assert!(artist.genres(connection).unwrap().is_empty());
    assert!(artist2.genres(connection).unwrap().is_empty());

    for (table, id) in vec![(Tables::Artists, artist.id), (Tables::Artists, artist2.id)] {
        let domain_events =
            DomainEvent::find(table, Some(id), Some(DomainEventTypes::GenresUpdated), connection).unwrap();
        assert_eq!(0, domain_events.len());
    }

    artist
        .set_genres(
            &vec!["emo".to_string(), "test".to_string(), "Hard Rock".to_string()],
            Some(creator.id),
            connection,
        )
        .unwrap();
    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert!(artist2.genres(connection).unwrap().is_empty());

    for (table, id, event_count) in vec![(Tables::Artists, artist.id, 1), (Tables::Artists, artist2.id, 0)] {
        let domain_events =
            DomainEvent::find(table, Some(id), Some(DomainEventTypes::GenresUpdated), connection).unwrap();
        assert_eq!(event_count, domain_events.len());
    }

    artist2
        .set_genres(
            &vec!["emo".to_string(), "happy".to_string()],
            Some(creator.id),
            connection,
        )
        .unwrap();
    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert_eq!(
        artist2.genres(connection).unwrap(),
        vec!["emo".to_string(), "happy".to_string()]
    );

    for (table, id, event_count) in vec![(Tables::Artists, artist.id, 1), (Tables::Artists, artist2.id, 1)] {
        let domain_events =
            DomainEvent::find(table, Some(id), Some(DomainEventTypes::GenresUpdated), connection).unwrap();
        assert_eq!(event_count, domain_events.len());
    }

    // Remove genre
    artist2
        .set_genres(&vec!["emo".to_string()], Some(creator.id), connection)
        .unwrap();
    assert_eq!(
        artist.genres(connection).unwrap(),
        vec!["emo".to_string(), "hard-rock".to_string(), "test".to_string()]
    );
    assert_eq!(artist2.genres(connection).unwrap(), vec!["emo".to_string()]);

    for (table, id, event_count) in vec![(Tables::Artists, artist.id, 1), (Tables::Artists, artist2.id, 2)] {
        let domain_events =
            DomainEvent::find(table, Some(id), Some(DomainEventTypes::GenresUpdated), connection).unwrap();
        assert_eq!(event_count, domain_events.len());
    }
}

#[test]
fn for_display() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();
    artist
        .set_genres(&vec!["emo".to_string(), "hard-rock".to_string()], None, connection)
        .unwrap();

    let display_artist = artist.clone().for_display(connection).unwrap();
    assert_eq!(display_artist.id, artist.id);
    assert_eq!(display_artist.name, artist.name);
    assert_eq!(display_artist.genres, vec!["emo".to_string(), "hard-rock".to_string()]);
}

#[test]
fn events() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    let event = project.create_event().with_name("Event 1".to_string()).finish();
    let event2 = project.create_event().with_name("Event 2".to_string()).finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist)
        .finish();
    project
        .create_event_artist()
        .with_event(&event)
        .with_artist(&artist2)
        .finish();
    project
        .create_event_artist()
        .with_event(&event2)
        .with_artist(&artist2)
        .finish();

    assert_eq!(artist.events(connection), Ok(vec![event.clone()]));
    assert_eq!(artist2.events(connection), Ok(vec![event, event2]));
}

#[test]
fn commit() {
    let project = TestProject::new();
    let name = "Name";
    let bio = "Bio";
    let website_url = "http://www.example.com";

    let artist = Artist::create(name, None, bio, website_url)
        .commit(project.get_connection())
        .unwrap();
    assert_eq!(name, artist.name);
    assert_eq!(bio, artist.bio);
    assert_eq!(website_url, artist.website_url.unwrap());
    assert_eq!(artist.id.to_string().is_empty(), false);
}

#[test]
fn new_artist_validate() {
    let name = "Name";
    let bio = "Bio";
    let website_url = "invalid.com";

    let mut artist = Artist::create(name, None, bio, website_url);

    artist.image_url = Some("invalid".into());
    artist.thumb_image_url = Some("invalid".into());

    artist.youtube_video_urls = Some(vec!["h".into()]);

    let result = artist.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("image_url"));
    assert_eq!(errors["image_url"].len(), 1);
    assert_eq!(errors["image_url"][0].code, "url");
    assert_eq!(
        &errors["image_url"][0].message.clone().unwrap().into_owned(),
        "Image URL is invalid"
    );

    assert!(errors.contains_key("youtube_video_urls"));
    assert_eq!(errors["youtube_video_urls"].len(), 1);
    assert_eq!(errors["youtube_video_urls"][0].code, "url");
    assert_eq!(
        &errors["youtube_video_urls"][0].message.clone().unwrap().into_owned(),
        "URL is invalid"
    );

    assert!(errors.contains_key("website_url"));
    assert_eq!(errors["website_url"].len(), 1);
    assert_eq!(errors["website_url"][0].code, "url");
    assert_eq!(
        &errors["website_url"][0].message.clone().unwrap().into_owned(),
        "Website URL is invalid"
    );
}

#[test]
fn artist_search() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let paging: &Paging = &Paging {
        page: 0,
        limit: 10,
        sort: "".to_string(),
        dir: SortingDir::Asc,
        total: 0,
        tags: HashMap::new(),
    };
    let owner = project.create_user().finish();
    let member = project.create_user().finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();

    let mut artists = vec![];
    artists.push(
        project
            .create_artist()
            .with_name("The artists".to_string())
            .with_organization(&organization)
            .finish(),
    );

    let sample_artists = vec!["More artists", "Too many artists"];

    for name in &sample_artists {
        artists.push(project.create_artist().with_name(name.to_string()).finish());
    }

    artists.push(
        project
            .create_artist()
            .with_name("Hidden artists".to_string())
            .with_organization(&organization)
            .make_private()
            .finish(),
    );

    // Logged out search
    let (found_artists, total) =
        Artist::search(&None, Some("many".to_string()), paging, connection).expect("No artists found");
    assert_eq!(1, total);
    assert!(found_artists.iter().any(|a| a.name == sample_artists[1]));

    //  Logged out search, no filter query
    let (found_artists, total) = Artist::search(&None, None, paging, connection).expect("No artists found");
    assert_eq!(3, total);
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }

    // Owner search, with filter query
    let (found_artists, total) =
        Artist::search(&Some(owner), Some("artist".to_string()), paging, connection).expect("No artists found");
    assert_eq!(4, total);
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }

    // Member search, no filter query
    let (found_artists, total) = Artist::search(&Some(member), None, paging, connection).expect("No artists found");
    assert_eq!(4, total);
    for name in &sample_artists {
        assert!(found_artists.iter().any(|a| a.name == *name));
    }
}

#[test]
fn new_artist_merge() {
    let mut artist1: NewArtist = Default::default();
    let bio = "Artist formally known as Default::default()";
    let test_url = "http://test.test".to_string();
    let artist2 = NewArtist {
        name: "Override".to_string(),
        bio: bio.to_string(),
        website_url: Some(test_url.clone()),
        image_url: Some(test_url.clone()),
        thumb_image_url: Some(test_url.clone()),
        youtube_video_urls: Some(vec![test_url.clone()]),
        facebook_username: Some("fbusername".to_string()),
        spotify_id: Some("fakespotify".to_string()),
        ..Default::default()
    };

    artist1.merge(artist2);
    assert_eq!(bio, artist1.bio);
    assert_eq!(test_url, artist1.website_url.unwrap());
    assert_eq!(test_url, artist1.image_url.unwrap());
    assert_eq!(test_url, artist1.thumb_image_url.unwrap());
    assert_eq!(test_url, artist1.youtube_video_urls.unwrap()[0]);
    assert_eq!("fbusername", artist1.facebook_username.unwrap());
    assert_eq!("fakespotify", artist1.spotify_id.unwrap());
}

#[test]
fn artist_editable_attributes_validate() {
    let mut artist_parameters = ArtistEditableAttributes::new();
    artist_parameters.name = Some("New Name".into());
    artist_parameters.bio = Some("Bio".into());
    artist_parameters.website_url = Some(Some("invalid.com".into()));
    artist_parameters.youtube_video_urls = Some(vec!["invalid".to_string()]);

    let result = artist_parameters.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err().field_errors();

    assert!(errors.contains_key("website_url"));
    assert_eq!(errors["website_url"].len(), 1);
    assert_eq!(errors["website_url"][0].code, "url");
    assert_eq!(
        &errors["website_url"][0].message.clone().unwrap().into_owned(),
        "Website URL is invalid"
    );

    assert!(errors.contains_key("youtube_video_urls"));
    assert_eq!(errors["youtube_video_urls"].len(), 1);
    assert_eq!(errors["youtube_video_urls"][0].code, "url");
    assert_eq!(
        &errors["youtube_video_urls"][0].message.clone().unwrap().into_owned(),
        "URL is invalid"
    );
}

#[test]
fn find() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();

    let found_artist = Artist::find(&artist.id, connection).expect("Artist was not found");
    assert_eq!(found_artist.id, artist.id);
    assert_eq!(found_artist.name, artist.name);

    assert!(
        match Artist::find(&Uuid::new_v4(), connection) {
            Ok(_artist) => false,
            Err(_e) => true,
        },
        "Artist incorrectly returned when id invalid"
    );
}

#[test]
fn set_privacy() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let mut artist = project.create_artist().finish();
    assert!(!artist.is_private);

    artist = artist.set_privacy(true, connection).unwrap();
    assert!(artist.is_private);

    artist = artist.set_privacy(false, connection).unwrap();
    assert!(!artist.is_private);
}

#[test]
fn all() {
    let project = TestProject::new();
    let conn = &project.get_connection();
    let name = "Name";
    let owner = project.create_user().finish();
    let user = project.create_user().finish();
    let admin = project.create_user().finish().add_role(Roles::Admin, conn).unwrap();

    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&user, Roles::OrgMember)
        .finish();

    let artist = project.create_artist().with_name(name.into()).finish();
    assert_eq!(name, artist.name);
    assert_eq!(artist.id.to_string().is_empty(), false);

    let found_artists = Artist::all(None, conn).unwrap();
    assert_eq!(1, found_artists.len());
    assert_eq!(found_artists[0].id, artist.id);
    assert_eq!(found_artists[0].name, artist.name);

    let name2 = "Name 2";
    let artist2 = project
        .create_artist()
        .with_name(name2.into())
        .with_organization(&organization)
        .make_private()
        .finish();
    assert_eq!(name2, artist2.name);
    assert_eq!(artist2.id.to_string().is_empty(), false);

    let found_artists = Artist::all(None, conn).unwrap();
    assert_eq!(1, found_artists.len());
    assert_eq!(found_artists[0].id, artist.id);
    assert_eq!(found_artists[0].name, artist.name);

    let found_artists_owner = Artist::all(Some(&owner), conn).unwrap();
    let found_artists_user = Artist::all(Some(&user), conn).unwrap();
    let found_artists_admin = Artist::all(Some(&admin), conn).unwrap();
    assert_eq!(2, found_artists_user.len());
    assert_eq!(2, found_artists_owner.len());
    assert_eq!(2, found_artists_admin.len());
    assert_eq!(found_artists_user[0].id, artist.id);
    assert_eq!(found_artists_user[0].name, artist.name);
    assert_eq!(found_artists_user[1].id, artist2.id);
    assert_eq!(found_artists_user[1].name, artist2.name);
    assert_eq!(found_artists_admin[1].id, artist2.id);
    assert_eq!(found_artists_admin[1].name, artist2.name);
}

#[test]
fn update() {
    let project = TestProject::new();
    let name = "Old Name";
    let artist = project.create_artist().with_name(name.into()).finish();

    let mut main_genre = Genre::find_or_create(&vec!["blues".to_string()], project.get_connection()).unwrap();
    let mut artist_parameters = ArtistEditableAttributes::new();
    artist_parameters.name = Some("New Name".into());
    artist_parameters.bio = Some("Bio".into());
    artist_parameters.website_url = Some(Some("http://www.example.com".into()));
    artist_parameters.main_genre_id = Some(main_genre.pop());

    let updated_artist = artist.update(&artist_parameters, &project.get_connection()).unwrap();

    assert_eq!(updated_artist.id, artist.id);
    assert_ne!(updated_artist.name, artist.name);
    assert_eq!(updated_artist.name, artist_parameters.name.unwrap());
    assert_eq!(updated_artist.main_genre_id, artist_parameters.main_genre_id.unwrap());
}

#[test]
fn destroy() {
    let project = TestProject::new();
    let artist = project.create_artist().finish();
    assert!(artist.destroy(project.get_connection()).unwrap() > 0);
    assert!(Artist::find(&artist.id, project.get_connection()).is_err());
}

#[test]
fn organization() {
    let project = TestProject::new();
    let organization = project.create_organization().finish();
    let artist = project.create_artist().with_organization(&organization).finish();
    let artist2 = project.create_artist().finish();

    assert_eq!(Ok(Some(organization)), artist.organization(project.get_connection()));
    assert_eq!(Ok(None), artist2.organization(project.get_connection()));
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
    let _public_artist = project.create_artist().with_name("Artist0".to_string()).finish();
    let organization = project
        .create_organization()
        .with_member(&owner, Roles::OrgOwner)
        .with_member(&member, Roles::OrgMember)
        .finish();
    let artist1 = project
        .create_artist()
        .with_name("Artist1".to_string())
        .with_organization(&organization)
        .finish()
        .for_display(connection)
        .unwrap();

    let artist2 = project
        .create_artist()
        .with_name("Artist2".to_string())
        .with_organization(&organization)
        .finish()
        .for_display(connection)
        .unwrap();

    let artist3 = project
        .create_artist()
        .with_name("Artist3".to_string())
        .with_organization(&organization)
        .make_private()
        .finish()
        .for_display(connection)
        .unwrap();

    // Add another artist for another org to make sure it isn't included
    let organization2 = project
        .create_organization()
        .with_member(&user, Roles::OrgOwner)
        .finish();
    let _artist4 = project
        .create_artist()
        .with_name("Artist4".to_string())
        .with_organization(&organization2)
        .finish()
        .for_display(connection)
        .unwrap();

    let user = project.create_user().finish();

    let public_organization_artists = vec![artist1.clone(), artist2.clone()];
    let organization_artists = vec![artist1, artist2, artist3];

    // Logged out / guest user
    let found_artists = Artist::find_for_organization(None, organization.id, connection).unwrap();
    assert_eq!(found_artists, public_organization_artists);

    // Private artist is not shown for users
    let found_artists = Artist::find_for_organization(Some(&user), organization.id, connection).unwrap();
    assert_eq!(found_artists, public_organization_artists);

    // Private artist is shown for admins, owners, and members
    let found_artists = Artist::find_for_organization(Some(&owner), organization.id, connection).unwrap();
    assert_eq!(found_artists, organization_artists);

    let found_artists = Artist::find_for_organization(Some(&member), organization.id, connection).unwrap();
    assert_eq!(found_artists, organization_artists);

    let found_artists = Artist::find_for_organization(Some(&admin), organization.id, connection).unwrap();
    assert_eq!(found_artists, organization_artists);
}
