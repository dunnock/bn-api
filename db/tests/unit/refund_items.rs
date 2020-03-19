use db::dev::TestProject;
use db::models::*;

#[test]
fn create() {
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

    assert_eq!(refund_item.refund_id, refund.id);
    assert_eq!(refund_item.order_item_id, order_item.id);
    assert_eq!(refund_item.amount, 10);
    assert_eq!(refund_item.quantity, 1);
}
