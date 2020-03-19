use db::dev::TestProject;
use db::models::*;

#[test]
fn create() {
    let project = TestProject::new();
    let order = project.create_order().finish();
    let user = project.create_user().finish();
    let refund = Refund::create(order.id, user.id, Some("Reasoning".to_string()), false)
        .commit(project.get_connection())
        .unwrap();

    assert_eq!(refund.order_id, order.id);
    assert_eq!(refund.user_id, user.id);
    assert_eq!(refund.reason, Some("Reasoning".to_string()));
    assert!(!refund.manual_override);
}

#[test]
fn find() {
    let project = TestProject::new();
    let refund = project.create_refund().finish();

    let found_refund = Refund::find(refund.id, project.get_connection()).unwrap();
    assert_eq!(refund, found_refund);
}

#[test]
fn items() {
    let project = TestProject::new();
    let connection = project.get_connection();
    let order = project.create_order().is_paid().finish();
    let items: Vec<OrderItem> = order
        .items(connection)
        .unwrap()
        .into_iter()
        .filter(|t| t.item_type == OrderItemTypes::Tickets)
        .collect();
    let order_item = &items[0];

    let refund = project.create_refund().finish();
    let refund_item = RefundItem::create(refund.id, order_item.id, 1, 10)
        .commit(connection)
        .unwrap();

    assert_eq!(vec![refund_item], refund.items(connection).unwrap());
}
