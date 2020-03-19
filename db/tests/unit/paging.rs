use db::models::*;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[test]
fn new() {
    let paging = Paging::new(1, 100);
    assert_eq!(paging.page, 1);
    assert_eq!(paging.limit, 100);
    assert_eq!(paging.sort, "".to_string());
    assert_eq!(paging.dir, SortingDir::Asc);
    assert_eq!(paging.total, 0);
    assert_eq!(paging.tags, HashMap::new());
}

#[test]
fn paging_parameters_into_paging() {
    let mut paging_parameters = PagingParameters::default();
    paging_parameters.page = Some(10);
    paging_parameters.limit = Some(100);
    paging_parameters.sort = Some("Test".to_string());
    paging_parameters.dir = Some(SortingDir::Asc);
    paging_parameters.tags = HashMap::new();

    let paging: Paging = paging_parameters.clone().into();
    assert_eq!(Some(paging.page), paging_parameters.page);
    assert_eq!(Some(paging.limit), paging_parameters.limit);
    assert_eq!(Some(paging.sort), paging_parameters.sort);
    assert_eq!(Some(paging.dir), paging_parameters.dir);
    assert_eq!(paging.tags, paging_parameters.tags);
    assert_eq!(paging.total, 0);
}

#[test]
fn payload_new() {
    let paging = Paging::new(1, 100);
    let payload = Payload::new(vec![Uuid::new_v4(), Uuid::new_v4()], paging);
    assert_eq!(payload.data.len(), 2);
}

#[test]
fn payload_from_data() {
    let uuids = vec![Uuid::new_v4(), Uuid::new_v4()];
    let payload = Payload::from_data(uuids, 1, 100, Some(2));
    assert_eq!(payload.data.len(), 2);
    assert_eq!(payload.paging.total, 2);
    assert!(!payload.is_empty());
}

#[test]
fn payload_empty() {
    let paging = Paging::new(1, 100);
    let payload: Payload<Uuid> = Payload::empty(paging);
    assert_eq!(payload.data.len(), 0);
    assert_eq!(payload.paging.total, 0);
    assert!(payload.is_empty());
}

#[test]
fn from_vector_to_payload() {
    let payload: Payload<Uuid> = vec![Uuid::new_v4(), Uuid::new_v4()].into();
    assert_eq!(payload.data.len(), 2);
    assert_eq!(payload.paging.total, 2);
    assert!(!payload.is_empty());
}

#[test]
fn page() {
    let mut paging_parameters = PagingParameters::default();
    assert_eq!(paging_parameters.page(), 0);

    paging_parameters.page = Some(2);
    assert_eq!(paging_parameters.page(), 2);
}

#[test]
fn limit() {
    let mut paging_parameters = PagingParameters::default();
    assert_eq!(paging_parameters.limit(), 100);

    paging_parameters.limit = Some(2);
    assert_eq!(paging_parameters.limit(), 2);
}

#[test]
fn dir() {
    let mut paging_parameters = PagingParameters::default();
    assert_eq!(paging_parameters.dir(), SortingDir::Asc);

    paging_parameters.dir = Some(SortingDir::Desc);
    assert_eq!(paging_parameters.dir(), SortingDir::Desc);
}

#[test]
fn get_tag() {
    let mut paging_parameters = PagingParameters::default();
    let mut tags: HashMap<String, Value> = HashMap::new();
    tags.insert("example".to_string(), json!("example-response"));
    paging_parameters.tags = tags;

    assert_eq!(paging_parameters.get_tag("test"), None);
    assert_eq!(
        paging_parameters.get_tag("example"),
        Some("example-response".to_string())
    );
}

#[test]
fn get_tag_as_str() {
    let mut paging_parameters = PagingParameters::default();
    let mut tags: HashMap<String, Value> = HashMap::new();
    tags.insert("example".to_string(), json!("example-response"));
    paging_parameters.tags = tags;

    assert_eq!(paging_parameters.get_tag_as_str("test"), None);
    assert_eq!(paging_parameters.get_tag_as_str("example"), Some("example-response"));
}

#[test]
fn query() {
    let mut paging_parameters = PagingParameters::default();
    let mut tags: HashMap<String, Value> = HashMap::new();
    tags.insert("query".to_string(), json!("example-response"));
    assert_eq!(paging_parameters.query(), None);
    paging_parameters.tags = tags;

    assert_eq!(paging_parameters.query(), Some("example-response"));
}
