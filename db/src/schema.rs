table! {
    analytics_page_views (id) {
        id -> Uuid,
        date -> Date,
        hour -> Time,
        event_id -> Uuid,
        source -> Text,
        medium -> Text,
        term -> Text,
        content -> Text,
        platform -> Text,
        campaign -> Text,
        url -> Text,
        code -> Text,
        client_id -> Text,
        user_agent -> Text,
        ip_address -> Text,
        count -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        referrer -> Text,
    }
}

table! {
    announcement_engagements (id) {
        id -> Uuid,
        user_id -> Uuid,
        announcement_id -> Uuid,
        action -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    announcements (id) {
        id -> Uuid,
        message -> Varchar,
        organization_id -> Nullable<Uuid>,
        deleted_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    artist_genres (id) {
        id -> Uuid,
        artist_id -> Uuid,
        genre_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    artists (id) {
        id -> Uuid,
        organization_id -> Nullable<Uuid>,
        is_private -> Bool,
        name -> Text,
        bio -> Text,
        image_url -> Nullable<Text>,
        thumb_image_url -> Nullable<Text>,
        website_url -> Nullable<Text>,
        youtube_video_urls -> Array<Text>,
        facebook_username -> Nullable<Text>,
        instagram_username -> Nullable<Text>,
        snapchat_username -> Nullable<Text>,
        soundcloud_username -> Nullable<Text>,
        bandcamp_username -> Nullable<Text>,
        spotify_id -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        other_image_urls -> Nullable<Array<Text>>,
        main_genre_id -> Nullable<Uuid>,
    }
}

table! {
    assets (id) {
        id -> Uuid,
        ticket_type_id -> Uuid,
        blockchain_name -> Text,
        blockchain_asset_id -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    broadcasts (id) {
        id -> Uuid,
        event_id -> Uuid,
        notification_type -> Varchar,
        channel -> Varchar,
        name -> Text,
        message -> Nullable<Text>,
        send_at -> Nullable<Timestamp>,
        status -> Varchar,
        progress -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        sent_quantity -> Int8,
        opened_quantity -> Int8,
        subject -> Nullable<Text>,
        audience -> Varchar,
        preview_email -> Nullable<Text>,
    }
}

table! {
    codes (id) {
        id -> Uuid,
        name -> Text,
        event_id -> Uuid,
        code_type -> Text,
        redemption_code -> Text,
        max_uses -> Int8,
        discount_in_cents -> Nullable<Int8>,
        start_date -> Timestamp,
        end_date -> Timestamp,
        max_tickets_per_user -> Nullable<Int8>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        discount_as_percentage -> Nullable<Int8>,
        deleted_at -> Nullable<Timestamp>,
    }
}

table! {
    domain_actions (id) {
        id -> Uuid,
        domain_event_id -> Nullable<Uuid>,
        domain_action_type -> Text,
        communication_channel_type -> Nullable<Text>,
        payload -> Json,
        main_table -> Nullable<Text>,
        main_table_id -> Nullable<Uuid>,
        scheduled_at -> Timestamp,
        expires_at -> Timestamp,
        last_attempted_at -> Nullable<Timestamp>,
        attempt_count -> Int8,
        max_attempt_count -> Int8,
        status -> Text,
        last_failure_reason -> Nullable<Text>,
        blocked_until -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    domain_event_published (domain_event_publisher_id, domain_event_id) {
        domain_event_publisher_id -> Uuid,
        domain_event_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    domain_event_publishers (id) {
        id -> Uuid,
        organization_id -> Nullable<Uuid>,
        event_types -> Array<Text>,
        webhook_url -> Text,
        import_historic_events -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_domain_event_seq -> Nullable<Int8>,
        deleted_at -> Nullable<Timestamp>,
        adapter -> Nullable<Varchar>,
        adapter_config -> Nullable<Jsonb>,
        blocked_until -> Timestamp,
    }
}

table! {
    domain_events (id) {
        id -> Uuid,
        event_type -> Text,
        display_text -> Text,
        event_data -> Nullable<Json>,
        main_table -> Text,
        main_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        user_id -> Nullable<Uuid>,
        seq -> Int8,
        organization_id -> Nullable<Uuid>,
    }
}

table! {
    event_artists (id) {
        id -> Uuid,
        event_id -> Uuid,
        artist_id -> Uuid,
        rank -> Int4,
        set_time -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        importance -> Int4,
        stage_id -> Nullable<Uuid>,
    }
}

table! {
    event_genres (id) {
        id -> Uuid,
        event_id -> Uuid,
        genre_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    event_interest (id) {
        id -> Uuid,
        event_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    event_report_subscribers (id) {
        id -> Uuid,
        event_id -> Uuid,
        email -> Text,
        report_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    events (id) {
        id -> Uuid,
        name -> Text,
        organization_id -> Uuid,
        venue_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        event_start -> Nullable<Timestamp>,
        door_time -> Nullable<Timestamp>,
        status -> Text,
        publish_date -> Nullable<Timestamp>,
        redeem_date -> Nullable<Timestamp>,
        promo_image_url -> Nullable<Text>,
        additional_info -> Nullable<Text>,
        age_limit -> Nullable<Varchar>,
        top_line_info -> Nullable<Varchar>,
        cancelled_at -> Nullable<Timestamp>,
        updated_at -> Timestamp,
        video_url -> Nullable<Text>,
        is_external -> Bool,
        external_url -> Nullable<Text>,
        override_status -> Nullable<Text>,
        client_fee_in_cents -> Nullable<Int8>,
        company_fee_in_cents -> Nullable<Int8>,
        settlement_amount_in_cents -> Nullable<Int8>,
        event_end -> Nullable<Timestamp>,
        sendgrid_list_id -> Nullable<Int8>,
        event_type -> Text,
        cover_image_url -> Nullable<Text>,
        private_access_code -> Nullable<Text>,
        facebook_pixel_key -> Nullable<Text>,
        deleted_at -> Nullable<Timestamp>,
        extra_admin_data -> Nullable<Jsonb>,
        slug_id -> Nullable<Uuid>,
        facebook_event_id -> Nullable<Text>,
        settled_at -> Nullable<Timestamp>,
        cloned_from_event_id -> Nullable<Uuid>,
    }
}

table! {
    event_users (id) {
        id -> Uuid,
        user_id -> Uuid,
        event_id -> Uuid,
        role -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    external_logins (id) {
        id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        site -> Text,
        access_token -> Text,
        external_user_id -> Text,
        updated_at -> Timestamp,
        scopes -> Array<Text>,
        deleted_at -> Nullable<Timestamp>,
    }
}

table! {
    fee_schedule_ranges (id) {
        id -> Uuid,
        fee_schedule_id -> Uuid,
        min_price_in_cents -> Int8,
        fee_in_cents -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        company_fee_in_cents -> Int8,
        client_fee_in_cents -> Int8,
    }
}

table! {
    fee_schedules (id) {
        id -> Uuid,
        name -> Text,
        version -> Int2,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        organization_id -> Uuid,
    }
}

table! {
    genres (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    holds (id) {
        id -> Uuid,
        name -> Text,
        parent_hold_id -> Nullable<Uuid>,
        event_id -> Uuid,
        redemption_code -> Nullable<Text>,
        discount_in_cents -> Nullable<Int8>,
        end_at -> Nullable<Timestamp>,
        max_per_user -> Nullable<Int8>,
        hold_type -> Text,
        ticket_type_id -> Uuid,
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
    }
}

table! {
    notes (id) {
        id -> Uuid,
        note -> Text,
        main_table -> Text,
        main_id -> Uuid,
        deleted_by -> Nullable<Uuid>,
        deleted_at -> Nullable<Timestamp>,
        created_by -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    order_items (id) {
        id -> Uuid,
        order_id -> Uuid,
        item_type -> Text,
        ticket_type_id -> Nullable<Uuid>,
        event_id -> Nullable<Uuid>,
        quantity -> Int8,
        unit_price_in_cents -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        ticket_pricing_id -> Nullable<Uuid>,
        fee_schedule_range_id -> Nullable<Uuid>,
        parent_id -> Nullable<Uuid>,
        hold_id -> Nullable<Uuid>,
        code_id -> Nullable<Uuid>,
        company_fee_in_cents -> Int8,
        client_fee_in_cents -> Int8,
        refunded_quantity -> Int8,
    }
}

table! {
    orders (id) {
        id -> Uuid,
        user_id -> Uuid,
        status -> Text,
        order_type -> Text,
        order_date -> Timestamp,
        expires_at -> Nullable<Timestamp>,
        version -> Int8,
        on_behalf_of_user_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        paid_at -> Nullable<Timestamp>,
        box_office_pricing -> Bool,
        checkout_url -> Nullable<Text>,
        checkout_url_expires -> Nullable<Timestamp>,
        create_user_agent -> Nullable<Text>,
        purchase_user_agent -> Nullable<Text>,
        external_payment_type -> Nullable<Text>,
        tracking_data -> Nullable<Jsonb>,
        source -> Nullable<Text>,
        campaign -> Nullable<Text>,
        medium -> Nullable<Text>,
        term -> Nullable<Text>,
        content -> Nullable<Text>,
        platform -> Nullable<Text>,
        settlement_id -> Nullable<Uuid>,
        referrer -> Nullable<Text>,
    }
}

table! {
    order_transfers (id) {
        id -> Uuid,
        order_id -> Uuid,
        transfer_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    organization_interactions (id) {
        id -> Uuid,
        organization_id -> Uuid,
        user_id -> Uuid,
        first_interaction -> Timestamp,
        last_interaction -> Timestamp,
        interaction_count -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    organization_invites (id) {
        id -> Uuid,
        organization_id -> Uuid,
        inviter_id -> Uuid,
        user_email -> Text,
        created_at -> Timestamp,
        security_token -> Nullable<Uuid>,
        user_id -> Nullable<Uuid>,
        accepted -> Nullable<Int2>,
        updated_at -> Timestamp,
        sent_invite -> Bool,
        roles -> Array<Text>,
        event_ids -> Array<Uuid>,
    }
}

table! {
    organizations (id) {
        id -> Uuid,
        name -> Text,
        address -> Nullable<Text>,
        city -> Nullable<Text>,
        state -> Nullable<Text>,
        country -> Nullable<Text>,
        postal_code -> Nullable<Text>,
        phone -> Nullable<Text>,
        event_fee_in_cents -> Int8,
        sendgrid_api_key -> Nullable<Text>,
        google_ga_key -> Nullable<Text>,
        facebook_pixel_key -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        fee_schedule_id -> Uuid,
        client_event_fee_in_cents -> Int8,
        company_event_fee_in_cents -> Int8,
        allowed_payment_providers -> Array<Text>,
        timezone -> Nullable<Text>,
        cc_fee_percent -> Float4,
        globee_api_key -> Nullable<Text>,
        max_instances_per_ticket_type -> Int8,
        max_additional_fee_in_cents -> Int8,
        settlement_type -> Text,
        slug_id -> Nullable<Uuid>,
        google_ads_conversion_id -> Nullable<Text>,
        google_ads_conversion_labels -> Array<Text>,
    }
}

table! {
    organization_users (id) {
        id -> Uuid,
        organization_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        role -> Array<Text>,
        additional_scopes -> Nullable<Jsonb>,
    }
}

table! {
    organization_venues (id) {
        id -> Uuid,
        organization_id -> Uuid,
        venue_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    payment_methods (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Text,
        is_default -> Bool,
        provider -> Text,
        provider_data -> Json,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    payments (id) {
        id -> Uuid,
        order_id -> Uuid,
        created_by -> Nullable<Uuid>,
        status -> Text,
        payment_method -> Text,
        amount -> Int8,
        provider -> Text,
        external_reference -> Nullable<Text>,
        raw_data -> Nullable<Json>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        url_nonce -> Nullable<Text>,
        refund_id -> Nullable<Uuid>,
    }
}

table! {
    push_notification_tokens (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_source -> Text,
        token -> Text,
        last_notification_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
    }
}

table! {
    refunded_tickets (id) {
        id -> Uuid,
        order_item_id -> Uuid,
        ticket_instance_id -> Uuid,
        fee_refunded_at -> Nullable<Timestamp>,
        ticket_refunded_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    refund_items (id) {
        id -> Uuid,
        refund_id -> Uuid,
        order_item_id -> Uuid,
        quantity -> Int8,
        amount -> Int8,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    refunds (id) {
        id -> Uuid,
        order_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        reason -> Nullable<Text>,
        settlement_id -> Nullable<Uuid>,
        manual_override -> Bool,
    }
}

table! {
    regions (id) {
        id -> Uuid,
        name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    settlement_adjustments (id) {
        id -> Uuid,
        settlement_id -> Uuid,
        amount_in_cents -> Int8,
        note -> Nullable<Text>,
        settlement_adjustment_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    settlement_entries (id) {
        id -> Uuid,
        settlement_id -> Uuid,
        event_id -> Uuid,
        ticket_type_id -> Nullable<Uuid>,
        face_value_in_cents -> Int8,
        revenue_share_value_in_cents -> Int8,
        online_sold_quantity -> Int8,
        fee_sold_quantity -> Int8,
        total_sales_in_cents -> Int8,
        settlement_entry_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    settlements (id) {
        id -> Uuid,
        organization_id -> Uuid,
        start_time -> Timestamp,
        end_time -> Timestamp,
        status -> Text,
        comment -> Nullable<Text>,
        only_finished_events -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    slugs (id) {
        id -> Uuid,
        slug -> Varchar,
        main_table -> Text,
        main_table_id -> Uuid,
        slug_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        title -> Nullable<Text>,
        description -> Nullable<Text>,
    }
}

table! {
    source_aliases (id) {
        id -> Uuid,
        from_source -> Text,
        from_medium -> Text,
        to_source -> Text,
        to_medium -> Text,
    }
}

table! {
    stages (id) {
        id -> Uuid,
        venue_id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        capacity -> Nullable<Int8>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    temporary_user_links (temporary_user_id, user_id) {
        temporary_user_id -> Uuid,
        user_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    temporary_users (id) {
        id -> Uuid,
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    ticket_instances (id) {
        id -> Uuid,
        asset_id -> Uuid,
        token_id -> Int4,
        hold_id -> Nullable<Uuid>,
        order_item_id -> Nullable<Uuid>,
        wallet_id -> Uuid,
        reserved_until -> Nullable<Timestamp>,
        redeem_key -> Nullable<Text>,
        status -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        redeemed_by_user_id -> Nullable<Uuid>,
        redeemed_at -> Nullable<Timestamp>,
        first_name_override -> Nullable<Text>,
        last_name_override -> Nullable<Text>,
        check_in_source -> Nullable<Text>,
    }
}

table! {
    ticket_pricing (id) {
        id -> Uuid,
        ticket_type_id -> Uuid,
        name -> Text,
        status -> Text,
        price_in_cents -> Int8,
        start_date -> Timestamp,
        end_date -> Timestamp,
        is_box_office_only -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        previous_ticket_pricing_id -> Nullable<Uuid>,
    }
}

table! {
    ticket_type_codes (id) {
        id -> Uuid,
        ticket_type_id -> Uuid,
        code_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    ticket_types (id) {
        id -> Uuid,
        event_id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        status -> Text,
        start_date -> Nullable<Timestamp>,
        end_date -> Nullable<Timestamp>,
        increment -> Int4,
        limit_per_person -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        price_in_cents -> Int8,
        cancelled_at -> Nullable<Timestamp>,
        parent_id -> Nullable<Uuid>,
        rank -> Int4,
        visibility -> Varchar,
        additional_fee_in_cents -> Int8,
        deleted_at -> Nullable<Timestamp>,
        end_date_type -> Text,
        web_sales_enabled -> Bool,
        box_office_sales_enabled -> Bool,
        app_sales_enabled -> Bool,
    }
}

table! {
    transfers (id) {
        id -> Uuid,
        source_user_id -> Uuid,
        destination_user_id -> Nullable<Uuid>,
        transfer_key -> Uuid,
        status -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        transfer_message_type -> Nullable<Text>,
        transfer_address -> Nullable<Text>,
        cancelled_by_user_id -> Nullable<Uuid>,
        direct -> Bool,
        destination_temporary_user_id -> Nullable<Uuid>,
    }
}

table! {
    transfer_tickets (id) {
        id -> Uuid,
        ticket_instance_id -> Uuid,
        transfer_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    user_genres (id) {
        id -> Uuid,
        user_id -> Uuid,
        genre_id -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Uuid,
        first_name -> Nullable<Text>,
        last_name -> Nullable<Text>,
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        profile_pic_url -> Nullable<Text>,
        thumb_profile_pic_url -> Nullable<Text>,
        cover_photo_url -> Nullable<Text>,
        hashed_pw -> Text,
        password_modified_at -> Timestamp,
        created_at -> Timestamp,
        last_used -> Nullable<Timestamp>,
        active -> Bool,
        role -> Array<Text>,
        password_reset_token -> Nullable<Uuid>,
        password_reset_requested_at -> Nullable<Timestamp>,
        updated_at -> Timestamp,
        last_cart_id -> Nullable<Uuid>,
        accepted_terms_date -> Nullable<Timestamp>,
        invited_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
    }
}

table! {
    venues (id) {
        id -> Uuid,
        region_id -> Nullable<Uuid>,
        is_private -> Bool,
        name -> Text,
        address -> Text,
        city -> Text,
        state -> Text,
        country -> Text,
        postal_code -> Text,
        phone -> Nullable<Text>,
        promo_image_url -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        google_place_id -> Nullable<Text>,
        latitude -> Nullable<Float8>,
        longitude -> Nullable<Float8>,
        timezone -> Text,
        slug_id -> Nullable<Uuid>,
    }
}

table! {
    wallets (id) {
        id -> Uuid,
        user_id -> Nullable<Uuid>,
        organization_id -> Nullable<Uuid>,
        name -> Text,
        secret_key -> Text,
        public_key -> Text,
        default_flag -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(announcement_engagements -> announcements (announcement_id));
joinable!(announcement_engagements -> users (user_id));
joinable!(announcements -> organizations (organization_id));
joinable!(artist_genres -> artists (artist_id));
joinable!(artist_genres -> genres (genre_id));
joinable!(artists -> genres (main_genre_id));
joinable!(artists -> organizations (organization_id));
joinable!(assets -> ticket_types (ticket_type_id));
joinable!(broadcasts -> events (event_id));
joinable!(codes -> events (event_id));
joinable!(domain_actions -> domain_events (domain_event_id));
joinable!(domain_event_published -> domain_event_publishers (domain_event_publisher_id));
joinable!(domain_event_published -> domain_events (domain_event_id));
joinable!(domain_event_publishers -> organizations (organization_id));
joinable!(domain_events -> organizations (organization_id));
joinable!(domain_events -> users (user_id));
joinable!(event_artists -> artists (artist_id));
joinable!(event_artists -> events (event_id));
joinable!(event_artists -> stages (stage_id));
joinable!(event_genres -> events (event_id));
joinable!(event_genres -> genres (genre_id));
joinable!(event_interest -> events (event_id));
joinable!(event_interest -> users (user_id));
joinable!(event_report_subscribers -> events (event_id));
joinable!(event_users -> events (event_id));
joinable!(event_users -> users (user_id));
joinable!(events -> organizations (organization_id));
joinable!(events -> venues (venue_id));
joinable!(external_logins -> users (user_id));
joinable!(fee_schedule_ranges -> fee_schedules (fee_schedule_id));
joinable!(holds -> events (event_id));
joinable!(holds -> ticket_types (ticket_type_id));
joinable!(order_items -> codes (code_id));
joinable!(order_items -> events (event_id));
joinable!(order_items -> fee_schedule_ranges (fee_schedule_range_id));
joinable!(order_items -> holds (hold_id));
joinable!(order_items -> orders (order_id));
joinable!(order_items -> ticket_pricing (ticket_pricing_id));
joinable!(order_items -> ticket_types (ticket_type_id));
joinable!(order_transfers -> orders (order_id));
joinable!(order_transfers -> transfers (transfer_id));
joinable!(orders -> settlements (settlement_id));
joinable!(organization_interactions -> organizations (organization_id));
joinable!(organization_interactions -> users (user_id));
joinable!(organization_invites -> organizations (organization_id));
joinable!(organization_users -> organizations (organization_id));
joinable!(organization_users -> users (user_id));
joinable!(organization_venues -> organizations (organization_id));
joinable!(organization_venues -> venues (venue_id));
joinable!(organizations -> fee_schedules (fee_schedule_id));
joinable!(payment_methods -> users (user_id));
joinable!(payments -> orders (order_id));
joinable!(payments -> refunds (refund_id));
joinable!(payments -> users (created_by));
joinable!(push_notification_tokens -> users (user_id));
joinable!(refund_items -> order_items (order_item_id));
joinable!(refund_items -> refunds (refund_id));
joinable!(refunded_tickets -> order_items (order_item_id));
joinable!(refunded_tickets -> ticket_instances (ticket_instance_id));
joinable!(refunds -> orders (order_id));
joinable!(refunds -> settlements (settlement_id));
joinable!(refunds -> users (user_id));
joinable!(settlement_adjustments -> settlements (settlement_id));
joinable!(settlement_entries -> events (event_id));
joinable!(settlement_entries -> settlements (settlement_id));
joinable!(settlement_entries -> ticket_types (ticket_type_id));
joinable!(settlements -> organizations (organization_id));
joinable!(temporary_user_links -> temporary_users (temporary_user_id));
joinable!(temporary_user_links -> users (user_id));
joinable!(ticket_instances -> assets (asset_id));
joinable!(ticket_instances -> holds (hold_id));
joinable!(ticket_instances -> order_items (order_item_id));
joinable!(ticket_instances -> wallets (wallet_id));
joinable!(ticket_pricing -> ticket_types (ticket_type_id));
joinable!(ticket_type_codes -> codes (code_id));
joinable!(ticket_type_codes -> ticket_types (ticket_type_id));
joinable!(ticket_types -> events (event_id));
joinable!(transfer_tickets -> ticket_instances (ticket_instance_id));
joinable!(transfer_tickets -> transfers (transfer_id));
joinable!(user_genres -> genres (genre_id));
joinable!(user_genres -> users (user_id));
joinable!(venues -> regions (region_id));
joinable!(wallets -> organizations (organization_id));
joinable!(wallets -> users (user_id));

allow_tables_to_appear_in_same_query!(
    analytics_page_views,
    announcement_engagements,
    announcements,
    artist_genres,
    artists,
    assets,
    broadcasts,
    codes,
    domain_actions,
    domain_event_published,
    domain_event_publishers,
    domain_events,
    event_artists,
    event_genres,
    event_interest,
    event_report_subscribers,
    events,
    event_users,
    external_logins,
    fee_schedule_ranges,
    fee_schedules,
    genres,
    holds,
    notes,
    order_items,
    orders,
    order_transfers,
    organization_interactions,
    organization_invites,
    organizations,
    organization_users,
    organization_venues,
    payment_methods,
    payments,
    push_notification_tokens,
    refunded_tickets,
    refund_items,
    refunds,
    regions,
    settlement_adjustments,
    settlement_entries,
    settlements,
    slugs,
    source_aliases,
    stages,
    temporary_user_links,
    temporary_users,
    ticket_instances,
    ticket_pricing,
    ticket_type_codes,
    ticket_types,
    transfers,
    transfer_tickets,
    user_genres,
    users,
    venues,
    wallets,
);
