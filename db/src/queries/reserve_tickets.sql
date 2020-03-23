WITH r AS (SELECT t.id
           FROM ticket_instances AS t
                    INNER JOIN assets AS a ON t.asset_id = a.id
           WHERE ((t.reserved_until < now() AND t.status = 'Reserved') OR t.status = 'Available')
             AND a.ticket_type_id = $3
             and t.parent_id is null
             AND coalesce($4, 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11') =
                 coalesce(t.hold_id, 'a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11') -- dummy guid
           LIMIT $5 FOR UPDATE OF t SKIP LOCKED)

UPDATE ticket_instances

SET order_item_id  = $1,
    reserved_until = $2,
    status         = 'Reserved',
    updated_at     = now()
FROM r
WHERE ticket_instances.id = r.id RETURNING ticket_instances.*;

