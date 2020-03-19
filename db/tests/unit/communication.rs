use db::prelude::*;

#[test]
fn new() {
    let communication_address = CommAddress::new();
    assert!(communication_address.addresses.is_empty());
}

#[test]
fn from() {
    let communication_address = CommAddress::from("abc@tari.com".to_string());
    assert_eq!(communication_address.addresses, vec!["abc@tari.com".to_string()]);
}

#[test]
fn from_vec() {
    let communication_address = CommAddress::from_vec(vec!["abc@tari.com".to_string(), "abc2@tari.com".to_string()]);
    assert_eq!(
        communication_address.addresses,
        vec!["abc@tari.com".to_string(), "abc2@tari.com".to_string()]
    );
}

#[test]
fn get() {
    let communication_address = CommAddress::from_vec(vec!["abc@tari.com".to_string(), "abc2@tari.com".to_string()]);
    assert_eq!(communication_address.addresses, communication_address.get());
}

#[test]
fn get_first() {
    let communication_address = CommAddress::from_vec(vec!["abc@tari.com".to_string(), "abc2@tari.com".to_string()]);
    assert_eq!(communication_address.get_first(), Ok("abc@tari.com".to_string()));

    let communication_address = CommAddress::new();
    assert_eq!(
        communication_address.get_first(),
        Err(DatabaseError::new(
            ErrorCode::BusinessProcessError,
            Some("Minimum of one communication address required".to_string()),
        ))
    );
}

#[test]
fn push() {
    let mut communication_address = CommAddress::from("abc@tari.com".to_string());
    communication_address.push(&"abc2@tari.com".to_string());
    assert_eq!(
        communication_address.addresses,
        vec!["abc@tari.com".to_string(), "abc2@tari.com".to_string()]
    );
}

#[test]
fn communication_new() {
    let communication_type = CommunicationType::EmailTemplate;
    let title = "Title".to_string();
    let body = Some("Body".to_string());
    let source_communication_address = Some(CommAddress::from("abc@tari.com".to_string()));
    let destination_communication_address = CommAddress::from("def@tari.com".to_string());
    let template_id = Some("TemplateId".to_string());
    let categories = Some(vec!["Category".to_string()]);

    let communication = Communication::new(
        communication_type,
        title.clone(),
        body.clone(),
        source_communication_address.clone(),
        destination_communication_address.clone(),
        template_id.clone(),
        None,
        categories.clone(),
        None,
    );

    assert_eq!(communication.comm_type, communication_type);
    assert_eq!(communication.title, title);
    assert_eq!(communication.body, body);
    assert_eq!(communication.source, source_communication_address);
    assert_eq!(communication.destinations, destination_communication_address);
    assert_eq!(communication.template_id, template_id);
    assert_eq!(communication.categories, categories);
}
