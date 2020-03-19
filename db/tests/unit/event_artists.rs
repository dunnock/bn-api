use db::dev::TestProject;
use db::prelude::*;

#[test]
fn clear_all_from_event() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let artist = project.create_artist().finish();
    let event = project.create_event().finish();
    let rank = 1;
    let importance = 0;

    EventArtist::create(event.id, artist.id, rank, None, importance, None)
        .commit(None, project.get_connection())
        .unwrap();
    assert!(!EventArtist::find_all_from_event(event.id, connection)
        .unwrap()
        .is_empty());

    EventArtist::clear_all_from_event(event.id, connection).unwrap();
    assert!(EventArtist::find_all_from_event(event.id, connection)
        .unwrap()
        .is_empty());
}

#[test]
fn create() {
    let project = TestProject::new();
    let artist = project.create_artist().finish();
    let event = project.create_event().finish();
    let rank = 1;
    let importance = 0;

    let event_artist = EventArtist::create(event.id, artist.id, rank, None, importance, None)
        .commit(None, project.get_connection())
        .unwrap();

    assert_eq!(event_artist.event_id, event.id, "Event foreign key does not match");
    assert_eq!(event_artist.artist_id, artist.id, "Artist foreign key does not match");
    assert_eq!(event_artist.rank, rank, "Artist rank is not correct");
    assert_eq!(event_artist.importance, importance, "Artist importance is not correct");
    assert_eq!(event_artist.id.to_string().is_empty(), false);
}

#[test]
fn find_all_by_event() {
    let project = TestProject::new();
    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    let event = project.create_event().finish();

    let event_artist1 = EventArtist::create(event.id, artist1.id, 1, None, 0, None)
        .commit(None, project.get_connection())
        .unwrap();
    let event_artist2 = EventArtist::create(event.id, artist2.id, 2, None, 1, None)
        .commit(None, project.get_connection())
        .unwrap();

    let result = EventArtist::find_all_from_event(event.id, project.get_connection()).unwrap();

    assert_equiv!(
        vec![
            DisplayEventArtist {
                artist: artist1,
                set_time: event_artist1.set_time,
                event_id: event_artist1.event_id,
                rank: event_artist1.rank,
                importance: event_artist1.importance,
                stage_id: event_artist1.stage_id,
            },
            DisplayEventArtist {
                artist: artist2,
                set_time: event_artist2.set_time,
                event_id: event_artist2.event_id,
                rank: event_artist2.rank,
                importance: event_artist2.importance,
                stage_id: event_artist2.stage_id,
            }
        ],
        result
    );
}

#[test]
fn find_all_by_events() {
    let project = TestProject::new();
    let artist1 = project.create_artist().finish();
    let artist2 = project.create_artist().finish();
    let artist3 = project.create_artist().finish();

    let event1 = project.create_event().finish();
    let event2 = project.create_event().finish();

    let event1_artist1 = EventArtist::create(event1.id, artist1.id, 1, None, 0, None)
        .commit(None, project.get_connection())
        .unwrap();
    let event1_artist2 = EventArtist::create(event1.id, artist2.id, 2, None, 1, None)
        .commit(None, project.get_connection())
        .unwrap();

    let event2_artist1 = EventArtist::create(event2.id, artist1.id, 1, None, 0, None)
        .commit(None, project.get_connection())
        .unwrap();
    let event2_artist2 = EventArtist::create(event2.id, artist2.id, 2, None, 1, None)
        .commit(None, project.get_connection())
        .unwrap();
    let event2_artist3 = EventArtist::create(event2.id, artist3.id, 3, None, 0, None)
        .commit(None, project.get_connection())
        .unwrap();

    let mut result = EventArtist::find_all_from_events(&vec![event1.id, event2.id], project.get_connection()).unwrap();

    assert_equiv!(
        vec![
            DisplayEventArtist {
                artist: artist1.clone(),
                set_time: event1_artist1.set_time,
                event_id: event1_artist1.event_id,
                rank: event1_artist1.rank,
                importance: event1_artist1.importance,
                stage_id: event1_artist1.stage_id,
            },
            DisplayEventArtist {
                artist: artist2.clone(),
                set_time: event1_artist2.set_time,
                event_id: event1_artist2.event_id,
                rank: event1_artist2.rank,
                importance: event1_artist2.importance,
                stage_id: event1_artist2.stage_id,
            }
        ],
        result.remove(&event1.id).unwrap()
    );

    assert_equiv!(
        vec![
            DisplayEventArtist {
                artist: artist1,
                set_time: event2_artist1.set_time,
                event_id: event2_artist1.event_id,
                rank: event2_artist1.rank,
                importance: event2_artist1.importance,
                stage_id: event2_artist1.stage_id,
            },
            DisplayEventArtist {
                artist: artist2,
                set_time: event2_artist2.set_time,
                event_id: event2_artist2.event_id,
                rank: event2_artist2.rank,
                importance: event2_artist2.importance,
                stage_id: event2_artist2.stage_id,
            },
            DisplayEventArtist {
                artist: artist3,
                set_time: event2_artist3.set_time,
                event_id: event2_artist3.event_id,
                rank: event2_artist3.rank,
                importance: event2_artist3.importance,
                stage_id: event2_artist3.stage_id,
            }
        ],
        result.remove(&event2.id).unwrap()
    );
}
