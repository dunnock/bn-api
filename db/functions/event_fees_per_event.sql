
--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
-- EVENT FEES PER EVENT
--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
DROP FUNCTION IF EXISTS event_fees_per_event("start" TIMESTAMP, "end" TIMESTAMP, group_by TEXT);
-- The group_by can be a combination of 'hold', 'ticket_type', 'ticket_pricing' or a combination: 'ticket_type|ticket_pricing|hold'
-- Default grouping is by event if no sub-group is defined
CREATE OR REPLACE FUNCTION event_fees_per_event("start" TIMESTAMP, "end" TIMESTAMP, group_by TEXT)
    RETURNS TABLE
    (
        organization_id               UUID,
        event_id                      UUID,
        event_start                   TIMESTAMP,
        per_order_company_online_fees BIGINT,
        per_order_client_online_fees  BIGINT,
        per_order_total_fees_in_cents BIGINT
    )
AS
$body$
SELECT e.organization_id           AS organization_id,
       e_.id                       AS event_id,
       e_.event_start              AS event_start,
       CAST(COALESCE(SUM((COALESCE(oi.company_fee_in_cents, 0) *
                          (COALESCE(oi.quantity, 0) - COALESCE(oi.refunded_quantity, 0))))
                         FILTER (WHERE p.is_box_office IS FALSE),
                     0) AS BIGINT) AS per_order_company_online_fees,
       CAST(COALESCE(SUM((COALESCE(oi.client_fee_in_cents, 0) *
                          (COALESCE(oi.quantity, 0) - COALESCE(oi.refunded_quantity, 0))))
                         FILTER (WHERE p.is_box_office IS FALSE),
                     0) AS BIGINT) AS per_order_client_online_fees,
       CAST(COALESCE(SUM((COALESCE(oi.unit_price_in_cents, 0) *
                          (COALESCE(oi.quantity, 0) - COALESCE(oi.refunded_quantity, 0))))
                         FILTER (WHERE p.is_box_office IS FALSE),
                     0) AS BIGINT) AS per_order_total_fees_in_cents
FROM order_items oi
         LEFT JOIN orders o on oi.order_id = o.id
         LEFT JOIN events e on oi.event_id = e.id
         LEFT JOIN (SELECT e_.id, e_.event_start FROM events e_ WHERE $3 LIKE '%event%') AS e_ ON e_.id = oi.event_id
         RIGHT JOIN (SELECT order_id,
                            CAST(ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.payment_method), ', ') LIKE
                                 '%External' AS BOOLEAN) AS is_box_office
                     FROM payments p
                     WHERE p.status = 'Completed'
                     GROUP BY p.payment_method, p.order_id) AS p on o.id = p.order_id

WHERE oi.item_type = 'EventFees'
  AND ($1 IS NULL OR o.paid_at >= $1)
  AND ($2 IS NULL OR o.paid_at <= $2)
GROUP BY e.organization_id, e_.id, e_.event_start;
$body$
    LANGUAGE SQL;
