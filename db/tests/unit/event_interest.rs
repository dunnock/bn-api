use db::dev::TestProject;
use db::models::{EventInterest, User};
use rand::prelude::*;
use uuid::Uuid;

#[test]
fn remove() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let user = project.create_user().finish();
    let event = project.create_event().finish();
    assert!(
        !EventInterest::find_interest_by_event_ids_for_user(&vec![event.id], user.id, connection,)
            .unwrap()
            .get(&event.id)
            .unwrap()
    );

    EventInterest::create(event.id, user.id).commit(connection).unwrap();
    assert!(
        EventInterest::find_interest_by_event_ids_for_user(&vec![event.id], user.id, connection,)
            .unwrap()
            .get(&event.id)
            .unwrap()
    );

    EventInterest::remove(event.id, user.id, connection).unwrap();
    assert!(
        !EventInterest::find_interest_by_event_ids_for_user(&vec![event.id], user.id, connection,)
            .unwrap()
            .get(&event.id)
            .unwrap()
    );
}

#[test]
fn create() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let event_interest = EventInterest::create(event.id, user.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(event_interest.user_id, user.id);
    assert_eq!(event_interest.event_id, event.id);
}

#[test]
fn find_interest_by_event_ids_for_user() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let event1 = project.create_event().finish();
    let event2 = project.create_event().finish();
    let event3 = project.create_event().finish();

    // User 1 has event interests in event 1 and event 3
    let user1 = project.create_user().finish();
    EventInterest::create(event1.id, user1.id).commit(connection).unwrap();
    EventInterest::create(event3.id, user1.id).commit(connection).unwrap();

    // User 2 has event interests in event 2
    let user2 = project.create_user().finish();
    EventInterest::create(event2.id, user2.id).commit(connection).unwrap();

    // User 3 has no event interests
    let user3 = project.create_user().finish();

    let all_event_ids = vec![event1.id, event2.id, event3.id];

    // User 1
    let found_interest =
        EventInterest::find_interest_by_event_ids_for_user(&all_event_ids, user1.id, connection).unwrap();
    assert!(found_interest.get(&event1.id).unwrap());
    assert!(!found_interest.get(&event2.id).unwrap());
    assert!(found_interest.get(&event3.id).unwrap());

    // User 2
    let found_interest =
        EventInterest::find_interest_by_event_ids_for_user(&all_event_ids, user2.id, connection).unwrap();
    assert!(!found_interest.get(&event1.id).unwrap());
    assert!(found_interest.get(&event2.id).unwrap());
    assert!(!found_interest.get(&event3.id).unwrap());

    // User 3
    let found_interest =
        EventInterest::find_interest_by_event_ids_for_user(&all_event_ids, user3.id, connection).unwrap();
    assert!(!found_interest.get(&event1.id).unwrap());
    assert!(!found_interest.get(&event2.id).unwrap());
    assert!(!found_interest.get(&event3.id).unwrap());
}

#[test]
fn total_interest() {
    let project = TestProject::new();
    let user1 = project.create_user().finish();
    let user2 = project.create_user().finish();
    let event = project.create_event().finish();

    let _event_interest1 = EventInterest::create(event.id, user1.id)
        .commit(project.get_connection())
        .unwrap();

    let _event_interest2 = EventInterest::create(event.id, user2.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        EventInterest::total_interest(event.id, project.get_connection()).unwrap(),
        2
    );
}

#[test]
fn user_interest() {
    let project = TestProject::new();
    let user = project.create_user().finish();
    let event = project.create_event().finish();

    let _event_interest1 = EventInterest::create(event.id, user.id)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(
        EventInterest::user_interest(event.id, user.id, project.get_connection()).unwrap(),
        true
    );
}

#[test]
fn list_interested_users() {
    let project = TestProject::new();
    let primary_event = project.create_event().finish();
    let secondary_event = project.create_event().finish();
    let primary_user = project.create_user().finish();
    let request_from_page: usize = 0;
    let request_limit: usize = 100;

    let result = EventInterest::list_interested_users(
        primary_event.id,
        primary_user.id,
        request_from_page as u32,
        request_limit as u32,
        project.get_connection(),
    )
    .unwrap();
    assert!(result.data.is_empty());

    //Create set of secondary users with interest in the primary and secondary event
    let n_secondary_users = 15;
    let mut rng = thread_rng();
    let p_event_interest_flag_list: Vec<u8> = (0..n_secondary_users).map(|_| rng.gen_range(0, 2)).collect(); //[0,1]
    let s_event_interest_flag_list: Vec<u8> = (0..n_secondary_users).map(|_| rng.gen_range(0, 2)).collect(); //[0,1]
    let mut secondary_user_list: Vec<User> = Vec::new();
    for _u_id in 0..n_secondary_users {
        secondary_user_list.push(project.create_user().finish());
    }
    //Let primary user show interest in primary event
    EventInterest::create(primary_event.id, primary_user.id)
        .commit(project.get_connection())
        .unwrap();
    //Let secondary users show interest in primary or/and secondary event
    let mut desired_user_id_completelist: Vec<Uuid> = Vec::new();
    for u_id in 0..n_secondary_users {
        if p_event_interest_flag_list[u_id] == 1 {
            //Set interest for primary event
            desired_user_id_completelist.push(secondary_user_list[u_id].id);
            let _secondary_event_interest = EventInterest::create(primary_event.id, secondary_user_list[u_id].id)
                .commit(project.get_connection())
                .unwrap();
        }
        if s_event_interest_flag_list[u_id] == 1 {
            //Set interest for secondary event
            let _secondary_event_interest = EventInterest::create(secondary_event.id, secondary_user_list[u_id].id)
                .commit(project.get_connection())
                .unwrap();
        }
    }
    desired_user_id_completelist.sort(); //Sort results for testing purposes
    if desired_user_id_completelist.len() > 0 {
        //Test1 - Normal Query of list of interested users for event, excluding primary user
        let result = EventInterest::list_interested_users(
            primary_event.id,
            primary_user.id,
            request_from_page as u32,
            request_limit as u32,
            project.get_connection(),
        )
        .unwrap();

        let n_sublist_entries = desired_user_id_completelist.len();
        for u_id in 0..n_sublist_entries {
            assert_eq!(desired_user_id_completelist[u_id], result.data[u_id].user_id);
        }
    }
}
