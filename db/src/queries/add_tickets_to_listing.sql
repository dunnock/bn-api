WITH r AS (SELECT t.id
           FROM ticket_instances AS t
                    INNER JOIN assets AS a ON t.asset_id = a.id
           WHERE (t.listing_id is null)
             AND t.status = 'Purchased'
             AND t.wallet_id = $1
             AND a.ticket_type_id = $2
           ORDER BY t.id
           LIMIT $3 FOR UPDATE OF t SKIP LOCKED)
UPDATE ticket_instances
SET listing_id    = $4,
    updated_at = now()
FROM r
WHERE ticket_instances.id = r.id RETURNING
    ticket_instances.*;

