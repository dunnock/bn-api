SELECT
  COUNT(*) OVER ()                                                                          AS total,
  tt.name                                                                                   AS ticket_type_name,
  CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER(WHERE ti.status = 'Redeemed'), 0) AS BIGINT)   AS scanned_count,
  CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER(WHERE ti.status = 'Purchased'), 0) AS BIGINT)  AS not_scanned_count
FROM ticket_types tt
JOIN assets a ON tt.id = a.ticket_type_id
LEFT JOIN ticket_instances ti ON a.id = ti.asset_id
-- Confirm this isn't a refunded redeemed (they keep their redeemed status and order association unlike normal refunds)
LEFT JOIN refunded_tickets rt ON rt.ticket_instance_id = ti.id AND ti.order_item_id = rt.order_item_id
WHERE tt.event_id = $1
AND tt.status <> 'Cancelled'
AND rt.id IS NULL
GROUP BY tt.name, tt.rank, tt.status
ORDER BY tt.rank;
