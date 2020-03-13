--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
-- TICKET COUNT PER TICKET TYPE
--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
-- Legacy function
DROP FUNCTION IF EXISTS ticket_count_per_ticket_type(event_id UUID, organization_id UUID, group_by_event_id BOOLEAN, group_by_organization_id BOOLEAN);
-- New function
DROP FUNCTION IF EXISTS ticket_count_per_ticket_type(event_id UUID, organization_id UUID, group_by TEXT);
-- group_by can be 'event', 'ticket_type', 'event|ticket_type'
-- Default grouping is by organization
CREATE OR REPLACE FUNCTION ticket_count_per_ticket_type(event_id UUID, organization_id UUID, group_by TEXT, run_hour TIME)
    RETURNS TABLE
    (
        organization_id                      UUID,
        event_id                             UUID,
        event_start                          TIMESTAMP,
        ticket_type_id                       UUID,
        ticket_name                          TEXT,
        ticket_status                        TEXT,
        event_name                           TEXT,
        organization_name                    TEXT,
        allocation_count_including_nullified BIGINT,
        allocation_count                     BIGINT,
        unallocated_count                    BIGINT,
        reserved_count                       BIGINT,
        redeemed_count                       BIGINT,
        purchased_count                      BIGINT,
        purchased_yesterday_count            BIGINT,
        comp_purchased_yesterday_count       BIGINT,
        nullified_count                      BIGINT,
        available_for_purchase_count         BIGINT,
        total_refunded_count                 BIGINT,
        comp_count                           BIGINT,
        comp_available_count                 BIGINT,
        comp_redeemed_count                  BIGINT,
        comp_purchased_count                 BIGINT,
        comp_reserved_count                  BIGINT,
        comp_nullified_count                 BIGINT,
        hold_count                           BIGINT,
        hold_available_count                 BIGINT,
        hold_redeemed_count                  BIGINT,
        hold_purchased_count                 BIGINT,
        hold_reserved_count                  BIGINT,
        hold_nullified_count                 BIGINT
    )
AS
$body$
SELECT o.id                                                                                      AS organization_id,
       e.id                                                                                      AS event_id,
       e.event_start                                                                             AS event_start,
       tt.id                                                                                     AS ticket_type_id,
       CASE WHEN tt.status = 'Cancelled' THEN concat(tt.name, ' (Cancelled)') ELSE tt.name END   AS ticket_name,
       tt.status                                                                                 AS ticket_status,
       e.name                                                                                    AS event_name,
       o.name                                                                                    AS organization_name,

       -- Total Ticket Count
       CAST(COALESCE(COUNT(DISTINCT ti.id), 0) AS BIGINT)                                        AS allocation_count_including_nullified,
       CAST(
           COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status != 'Nullified'), 0) AS BIGINT) AS allocation_count,
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status = 'Available' OR
                                                (ti.status = 'Reserved' AND ti.reserved_until < NOW())),
                     0) AS BIGINT)                                                      AS unallocated_count,

       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status = 'Reserved' AND ti.reserved_until > NOW()),
                     0) AS BIGINT)                                                      AS reserved_count,
       CAST(
           COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status = 'Redeemed' AND rt2.id IS NULL), 0) AS BIGINT)   AS redeemed_count,
       CAST(
           COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status in ('Purchased', 'Redeemed') AND rt2.id IS NULL), 0) AS BIGINT)  AS purchased_count,
       CAST(
         COALESCE(COUNT(DISTINCT ti.id) FILTER (
              WHERE ti.status in ('Purchased', 'Redeemed')
              AND rt2.id IS NULL
              AND CASE WHEN CURRENT_DATE + run_hour < now()
                THEN o2.paid_at >= CURRENT_DATE - 1 + run_hour
                ELSE o2.paid_at >= CURRENT_DATE - 2 + run_hour END
              AND CASE WHEN CURRENT_DATE + run_hour < now()
                THEN o2.paid_at < CURRENT_DATE + run_hour
                ELSE o2.paid_at < CURRENT_DATE - 1 + run_hour END
         ), 0) AS BIGINT)  AS purchased_yesterday_count,
       CAST(
         COALESCE(COUNT(DISTINCT ti.id) FILTER (
              WHERE ti.hold_id IS NOT NULL
              AND rt2.id IS NULL
              AND h.hold_type = 'Comp'
              AND ti.status in ('Purchased', 'Redeemed')
              AND CASE WHEN CURRENT_DATE + run_hour < now()
                THEN o2.paid_at >= CURRENT_DATE - 1 + run_hour
                ELSE o2.paid_at >= CURRENT_DATE - 2 + run_hour END
              AND CASE WHEN CURRENT_DATE + run_hour < now()
                THEN o2.paid_at < CURRENT_DATE + run_hour
                ELSE o2.paid_at < CURRENT_DATE - 1 + run_hour END
         ), 0) AS BIGINT)  AS comp_purchased_yesterday_count,
       CAST(
           COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.status = 'Nullified'), 0) AS BIGINT)  AS nullified_count,
       -- Not in a hold and not purchased / reserved / redeemed etc
       -- What can a generic user purchase.
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NULL AND (ti.status = 'Available' OR
                                                                        (ti.status = 'Reserved' AND ti.reserved_until < NOW()))),
                     0) AS BIGINT)                                                      AS available_for_purchase_count,
       --Refunded
       CAST(COUNT(DISTINCT rt.id) AS BIGINT)                                                     AS total_refunded_count,
       -------------------- COMPS --------------------
       -- Comp counts
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp'),
                     0) AS BIGINT)                                                      AS comp_count,
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND
                                                (ti.status = 'Available' OR
                                                 (ti.status = 'Reserved' AND ti.reserved_until < NOW()))),
                     0) AS BIGINT)                                                      AS comp_available_count,
       -- comp_count - comp_available_count = the sum of these
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Redeemed' AND rt2.id IS NULL),
           0) AS BIGINT)                                                                AS comp_redeemed_count,
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status in ('Purchased', 'Redeemed') AND rt2.id IS NULL),
           0) AS BIGINT)                                                                AS comp_purchased_count,
       CAST(COALESCE(COUNT(DISTINCT ti.id)
                           FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Reserved' AND
                                         ti.reserved_until > NOW()),
                     0) AS BIGINT)                                                      AS comp_reserved_count,
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type = 'Comp' AND ti.status = 'Nullified'),
           0) AS BIGINT)                                                                AS comp_nullified_count,
       ------------------ END COMPS ------------------

       -------------------- HOLDS --------------------
       -- Hold Counts
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp'),
                     0) AS BIGINT)                                                      AS hold_count,
       CAST(COALESCE(COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND
                                                (ti.status = 'Available' OR
                                                 (ti.status = 'Reserved' AND ti.reserved_until < NOW()))),
                     0) AS BIGINT)                                                      AS hold_available_count,
       -- hold_count - hold_available_count = the sum of these
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Redeemed' AND rt2.id IS NULL),
           0) AS BIGINT)                                                                AS hold_redeemed_count,
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status in ('Purchased', 'Redeemed') AND rt2.id IS NULL),
           0) AS BIGINT)                                                                AS hold_purchased_count,
       CAST(COALESCE(COUNT(DISTINCT ti.id)
                           FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Reserved' AND
                                         ti.reserved_until > NOW()),
                     0) AS BIGINT)                                                      AS hold_reserved_count,
       CAST(COALESCE(
           COUNT(DISTINCT ti.id) FILTER (WHERE ti.hold_id IS NOT NULL AND h.hold_type != 'Comp' AND ti.status = 'Nullified'),
           0) AS BIGINT)                                                                AS hold_nullified_count
       ------------------ END HOLDS -------------------
FROM ticket_instances ti
         LEFT JOIN holds h ON (h.id = ti.hold_id)
         LEFT JOIN assets a ON (a.id = ti.asset_id)
         LEFT JOIN (SELECT tt.id, tt.name, tt.status FROM ticket_types tt WHERE $3 LIKE '%ticket_type%') AS tt
                   ON tt.id = a.ticket_type_id
         LEFT JOIN ticket_types tt2 ON a.ticket_type_id = tt2.id
         LEFT JOIN (SELECT e.id, e.organization_id, e.name, e.event_start
                    FROM events e
                    WHERE $3 SIMILAR TO '%event%|%ticket_type%') AS e ON (e.id = tt2.event_id)
         LEFT JOIN events e2 ON (e2.id = tt2.event_id)
         LEFT JOIN organizations o ON o.id = e2.organization_id
         LEFT JOIN order_items oi ON (oi.id = ti.order_item_id)
         LEFT JOIN orders o2 ON (o2.id = oi.order_id)
         LEFT JOIN refunded_tickets rt ON (ti.id = rt.ticket_instance_id)
         LEFT JOIN refunded_tickets rt2 ON (ti.id = rt2.ticket_instance_id AND ti.order_item_id = rt2.order_item_id)
WHERE ($1 IS NULL OR e2.id = $1)
  AND ($2 IS NULL OR e2.organization_id = $2)
  AND (tt2.deleted_at IS NULL)
  AND (e2.deleted_at IS NULL)
GROUP BY e.id, e.event_start, e.name, o.id, o.name, tt2.rank, tt.id, tt.name, tt.status
ORDER BY e.id, e.name, o.id, o.name, tt2.rank;
$body$
    LANGUAGE SQL;

--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
