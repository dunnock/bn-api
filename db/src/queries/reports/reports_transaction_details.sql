SELECT COUNT(*) OVER ()                                                                                   AS total,
    e.name                                                                                             AS event_name,
    CASE WHEN tt.status = 'Cancelled' THEN concat(tt.name, ' (Cancelled)') ELSE tt.name END            AS ticket_name,
    CAST(oi.quantity AS BIGINT)                                                                        AS quantity,
    CAST(COALESCE(oi.refunded_quantity, 0) AS BIGINT)                                                  AS refunded_quantity,
    CAST(oi.quantity - COALESCE(oi.refunded_quantity, 0) AS BIGINT)                                    AS actual_quantity,
    CAST(oi.unit_price_in_cents AS BIGINT)                                                             AS unit_price_in_cents,
    CAST(oi.unit_price_in_cents AS BIGINT)
    + CAST(COALESCE(oi_promo_code.unit_price_in_cents, 0) AS BIGINT)                                   AS face_price_in_cents,
    CAST((oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0))
    * (COALESCE(oi.quantity, 0) - COALESCE(oi.refunded_quantity, 0)) AS BIGINT)                        AS face_price_in_cents_total,
    CAST(((oi.unit_price_in_cents + COALESCE(oi_promo_code.unit_price_in_cents, 0))
    * (COALESCE(oi.quantity, 0) - COALESCE(oi.refunded_quantity, 0))
    + (COALESCE(oi_fees.client_fee_in_cents, 0) *
    (COALESCE(oi_fees.quantity, 0) - COALESCE(oi_fees.refunded_quantity, 0)))) AS BIGINT)              AS gross,
    CAST(COALESCE(oi_fees.client_fee_in_cents, 0) AS BIGINT)                                           AS client_fee_in_cents,
    CAST(
           COALESCE(oi_fees.client_fee_in_cents, 0) *
           (COALESCE(oi_fees.quantity, 0) -
            COALESCE(oi_fees.refunded_quantity, 0)) AS BIGINT)                                        AS client_fee_in_cents_total,
    -- event fees
    CAST(COALESCE(oi_event_fees.client_fee_in_cents, 0) AS BIGINT)                                     AS event_fee_client_in_cents,
    CAST(
           COALESCE(oi_event_fees.client_fee_in_cents, 0) *
           (COALESCE(oi_event_fees.quantity, 0) -
            COALESCE(oi_event_fees.refunded_quantity, 0)) AS BIGINT)                                  AS event_fee_client_in_cents_total,
    oi_fees.fee_schedule_range_id                                                                      AS fee_range_id,
    o.paid_at                                                                                          AS transaction_date,
    o.order_type,
    p.payment_method,
    p.payment_provider,
    h.redemption_code,
    o.id                                                                                               AS order_id,
    oi.event_id,
    o.user_id,
    COALESCE(u.first_name, '')                                                                         AS first_name,
    COALESCE(u.last_name, '')                                                                          AS last_name,
    COALESCE(u.phone, '')                                                                              AS phone,
    COALESCE(u.email, '')                                                                              AS email,
    e.event_start                                                                                      AS event_start,
    CAST(COALESCE(oi_promo_code.unit_price_in_cents, 0) AS BIGINT)                                     AS promo_discount_value_in_cents,
    CAST(COALESCE(oi_promo_code.quantity, 0) - COALESCE(oi_promo_code.refunded_quantity, 0) AS BIGINT) AS promo_quantity,
    c.name                                                                                             AS promo_code_name,
    c.redemption_code                                                                                  AS promo_redemption_code,
    o.source,
    o.medium,
    o.campaign,
    o.term,
    o.content,
    o.platform,
    ti_agg.check_in_source,
    g.headline_artist_alt_genres,
    g.headline_artist_main_genre
FROM orders o
    LEFT JOIN order_items oi ON (o.id = oi.order_id AND oi.item_type = 'Tickets')
    LEFT JOIN order_items oi_fees ON (oi_fees.item_type = 'PerUnitFees' AND oi.id = oi_fees.parent_id)
    LEFT JOIN order_items oi_event_fees
        ON (oi_event_fees.item_type = 'EventFees' AND o.id = oi_event_fees.order_id)
    LEFT JOIN order_items oi_promo_code
        ON (oi_promo_code.item_type = 'Discount' AND oi.id = oi_promo_code.parent_id)
    LEFT JOIN codes c ON oi.code_id = c.id
    LEFT JOIN ticket_types tt ON (oi.ticket_type_id = tt.id)
    LEFT JOIN (SELECT order_id,
        ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.payment_method), ', ') AS payment_method,
        ARRAY_TO_STRING(ARRAY_AGG(DISTINCT p.provider), ', ')       AS payment_provider
        FROM payments p
        WHERE p.status IN ('Completed','Refunded')
        GROUP BY p.order_id) AS p on o.id = p.order_id
    LEFT JOIN (SELECT ti.order_item_id,
        NULLIF(ARRAY_TO_STRING(ARRAY_AGG(DISTINCT ti.check_in_source), ', '), '') AS check_in_source
        FROM ticket_instances ti
        GROUP BY ti.order_item_id) AS ti_agg ON ti_agg.order_item_id = oi.id
    LEFT JOIN holds h ON oi.hold_id = h.id
    LEFT JOIN events e ON oi.event_id = e.id
    LEFT JOIN users u ON COALESCE(o.on_behalf_of_user_id, o.user_id) = u.id
    LEFT JOIN (SELECT csvg.headline_artist_alt_genres, ea.event_id, g.name AS headline_artist_main_genre
        FROM event_artists ea
        LEFT JOIN (SELECT string_agg(g.name, ',') AS headline_artist_alt_genres, ag.artist_id
		FROM artist_genres ag
		INNER JOIN genres g ON ag.genre_id = g.id
		GROUP BY ag.artist_id) AS csvg ON ea.artist_id = csvg.artist_id
	INNER JOIN artists a on ea.artist_id = a.id
	LEFT JOIN genres g on a.main_genre_id = g.id
	WHERE ea.rank = 0
    ) AS g ON oi.event_id = g.event_id
WHERE o.status = 'Paid'
  AND ($1 IS NULL OR oi.event_id = $1)
  AND ($2 IS NULL OR e.organization_id = $2)
  AND ($3 IS NULL OR o.paid_at >= $3)
  AND ($4 IS NULL OR o.paid_at <= $4)
  AND (oi.item_type = 'Tickets')
  AND (
        $5 IS NULL
        OR u.email ILIKE concat('%', $5, '%')
        OR concat(u.first_name, ' ', u.last_name) ILIKE concat('%', $5, '%')
        OR o.id::text ILIKE concat('%', $5) -- matches end of id for order number
        OR e.name ILIKE concat('%', $5, '%')
    )
ORDER BY o.paid_at DESC
LIMIT $7
    OFFSET $6;
