WITH r AS (SELECT t.id
           FROM ticket_instances AS t
                    INNER JOIN assets AS a
                        inner join ticket_types as tt
                          inner join rarities r2
                            on tt.rarity_id = r2.id
                          on a.ticket_type_id = tt.id
                      ON t.asset_id = a.id
           WHERE ((t.reserved_until < now() AND t.status = 'Reserved') OR t.status = 'Available')
             AND tt.event_id = $2

            and ($3 is null or tt.id = $3)
             and ($4 is null or r2.rank >= (select min(rank) from rarities where id = $4)) -- messy, TODO: get this value in the code
            and ($5 is null or r2.rank <= (select max(rank) from rarities where id =$5)) -- same
             AND t.hold_id is null -- Can't steal from the hold
            and t.parent_id is null -- Not in another loot box
           ORDER BY t.status, t.reserved_until,  -- Grab available tickets first, then old reserved
           t.id -- some randomness, but not great.
           LIMIT $6 FOR UPDATE OF t SKIP LOCKED)
UPDATE ticket_instances
SET parent_id    = $1,
    updated_at = now()
FROM r
WHERE ticket_instances.id = r.id RETURNING
    ticket_instances.*;

