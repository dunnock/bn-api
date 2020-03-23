WITH cte AS (SELECT t.id
             FROM ticket_instances AS t
                      INNER JOIN assets AS a ON t.asset_id = a.id
             WHERE t.listing_id = $1
               AND a.ticket_type_id = $2
               AND t.status ='Purchased'
             ORDER BY t.created_at
             LIMIT $3 FOR UPDATE OF t SKIP LOCKED)
UPDATE ticket_instances
SET listing_id    = NULL,
    updated_at = now()
FROM cte
WHERE cte.id = ticket_instances.id RETURNING ticket_instances.*;

