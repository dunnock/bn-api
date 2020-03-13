use crate::controllers::*;
use crate::middleware::*;
use actix_web::web;
use bigneon_db::models::Scopes;

pub fn routes(app: &mut web::ServiceConfig) {
    // Please try to keep in alphabetical order

    app.service(
        web::resource("/admin/stuck_domain_actions").route(web::get().to(admin::admin::admin_stuck_domain_actions)),
    )
    .service(web::resource("/admin/ticket_count").route(web::get().to(admin::admin::admin_ticket_count)))
    .service(web::resource("/admin/orders").route(web::get().to(admin::admin::orders)))
    .service(web::resource("/admin/reports").route(web::get().to(admin::reports::get_report)))
    .service(web::resource("/a/t").route(web::get().to(analytics::track)))
    .service(
        web::resource("/artists/search")
            .wrap(CacheResourceTransform::new(CacheUsersBy::AnonymousOnly))
            .route(web::get().to(artists::search)),
    )
    .service(web::resource("/artists/{id}/toggle_privacy").route(web::put().to(artists::toggle_privacy)))
    .service(
        web::resource("/artists/{id}")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(artists::show))
            .route(web::put().to(artists::update)),
    )
    .service(
        web::resource("/artists")
            .wrap(CacheResourceTransform::new(CacheUsersBy::AnonymousOnly))
            .route(web::get().to(artists::index))
            .route(web::post().to(artists::create)),
    )
    .service(web::resource("/auth/token").route(web::post().to(auth::token)))
    .service(web::resource("/auth/token/refresh").route(web::post().to(auth::token_refresh)))
    .service(
        web::resource("/broadcasts/{id}")
            .route(web::get().to(broadcasts::show))
            .route(web::put().to(broadcasts::update))
            .route(web::delete().to(broadcasts::delete)),
    )
    .service(web::resource("/broadcasts/{id}/tracking_count").route(web::post().to(broadcasts::tracking_count)))
    .service(
        web::resource("/cart")
            .route(web::delete().to(cart::destroy))
            .route(web::post().to(cart::update_cart))
            .route(web::put().to(cart::replace_cart))
            .route(web::get().to(cart::show)),
    )
    .service(web::resource("/cart/{id}/duplicate").route(web::post().to(cart::duplicate)))
    .service(web::resource("/cart/clear_invalid_items").route(web::delete().to(cart::clear_invalid_items)))
    .service(web::resource("/cart/checkout").route(web::post().to(cart::checkout)))
    .service(web::resource("/codes/{id}/link").route(web::get().to(codes::link)))
    .service(
        web::resource("/codes/{id}")
            .route(web::get().to(codes::show))
            .route(web::put().to(codes::update))
            .route(web::delete().to(codes::destroy)),
    )
    .service(
        web::resource("/comps/{id}")
            .route(web::get().to(comps::show))
            .route(web::patch().to(comps::update))
            .route(web::delete().to(comps::destroy)),
    )
    .service(web::resource("/event_report_subscribers/{id}").route(web::delete().to(event_report_subscribers::destroy)))
    .service(
        web::resource("/events")
        // In future it may be better to cache this for every user to save the database hit
        .wrap(CacheResourceTransform::new(CacheUsersBy::GlobalRoles))
        .route(web::get().to(events::index))
        .route(web::post().to(events::create)),
    )
    .service(web::resource("/events/checkins").route(web::get().to(events::checkins)))
    .service(
        web::resource("/events/{id}")
        // In future it may be better to cache this for every user to save the database hit
        .wrap(CacheResourceTransform::new(CacheUsersBy::GlobalRoles))
        .route(web::get().to(events::show))
        .route(web::put().to(events::update))
        .route(web::delete().to(events::cancel)),
    )
    .service(web::resource("/events/{id}/delete").route(web::delete().to(events::delete)))
    .service(
        web::resource("/events/{id}/artists")
            .route(web::post().to(events::add_artist))
            .route(web::put().to(events::update_artists)),
    )
    .service(web::resource("/events/{id}/ticket_holder_count").route(web::get().to(events::ticket_holder_count)))
    .service(web::resource("/events/{id}/clone").route(web::post().to(events::clone)))
    .service(
        web::resource("/events/{id}/codes")
            .route(web::get().to(events::codes))
            .route(web::post().to(codes::create)),
    )
    .service(web::resource("/events/{id}/dashboard").route(web::get().to(events::dashboard)))
    .service(web::resource("/events/{id}/guests").route(web::get().to(events::guest_list)))
    .service(
        web::resource("/events/{id}/holds")
            .route(web::post().to(holds::create))
            .route(web::get().to(events::holds)),
    )
    .service(
        web::resource("/events/{id}/interest")
            .route(web::get().to(events::list_interested_users))
            .route(web::post().to(events::add_interest))
            .route(web::delete().to(events::remove_interest)),
    )
    .service(web::resource("/events/{id}/publish").route(web::post().to(events::publish)))
    .service(
        web::resource("/events/{id}/broadcasts")
            .route(web::post().to(broadcasts::create))
            .route(web::get().to(broadcasts::index))
            .route(web::put().to(broadcasts::update)),
    )
    .service(web::resource("/events/{id}/links").route(web::post().to(events::create_link)))
    .service(web::resource("/events/{id}/redeem/{ticket_instance_id}").route(web::post().to(events::redeem_ticket)))
    .service(web::resource("/events/{id}/redeem").route(web::post().to(events::redeem_ticket)))
    .service(
        web::resource("/events/{id}/report_subscribers")
            .route(web::get().to(event_report_subscribers::index))
            .route(web::post().to(event_report_subscribers::create)),
    )
    .service(web::resource("/events/{id}/tickets").route(web::get().to(tickets::index)))
    .service(
        web::resource("/events/{id}/ticket_types")
            .route(web::get().to(ticket_types::index))
            .route(web::post().to(ticket_types::create)),
    )
    .service(web::resource("/events/{id}/ticket_types/multiple").route(web::post().to(ticket_types::create_multiple)))
    .service(
        web::resource("/events/{event_id}/ticket_types/{ticket_type_id}")
            .route(web::patch().to(ticket_types::update))
            .route(web::delete().to(ticket_types::cancel)),
    )
    .service(web::resource("/events/{id}/unpublish").route(web::post().to(events::unpublish)))
    .service(web::resource("/events/{id}/users").route(web::get().to(events::users)))
    .service(web::resource("/events/{id}/users/invites").route(web::post().to(organization_invites::create_for_event)))
    .service(
        web::resource("/events/{id}/users/invites/{invite_id}").route(web::delete().to(organization_invites::destroy)),
    )
    .service(web::resource("/events/{id}/users/{user_id}").route(web::delete().to(events::remove_user)))
    .service(web::resource("/events/{id}/websockets").route(web::get().to(websockets::initate)))
    .service(web::resource("/external/facebook/pages").route(web::get().to(external::facebook::pages)))
    .service(web::resource("/external/facebook/events").route(web::post().to(external::facebook::create_event)))
    .service(web::resource("/external/facebook/web_login").route(web::post().to(external::facebook::web_login)))
    .service(web::resource("/external/facebook/scopes").route(web::get().to(external::facebook::scopes)))
    .service(web::resource("/external/facebook").route(web::delete().to(external::facebook::disconnect)))
    .service(
        web::resource("/genres")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(genres::index)),
    )
    .service(web::resource("/invitations/{id}").route(web::get().to(organization_invites::view)))
    .service(web::resource("/invitations").route(web::post().to(organization_invites::accept_request)))
    .service(web::resource("/ipns/globee").route(web::post().to(ipns::globee)))
    .service(
        web::resource("/holds/{id}/comps")
            .route(web::get().to(comps::index))
            .route(web::post().to(comps::create)),
    )
    .service(web::resource("/holds/{id}/split").route(web::post().to(holds::split)))
    .service(web::resource("/holds/{id}/children").route(web::get().to(holds::children)))
    .service(web::resource("/holds/{id}/link").route(web::get().to(holds::link)))
    .service(
        web::resource("/holds/{id}")
            .route(web::patch().to(holds::update))
            .route(web::get().to(holds::show))
            .route(web::delete().to(holds::destroy)),
    )
    .service(web::resource("/notes/{id}").route(web::delete().to(notes::destroy)))
    .service(
        web::resource("/notes/{main_table}/{id}")
            .route(web::get().to(notes::index))
            .route(web::post().to(notes::create)),
    )
    .service(web::resource("/orders").route(web::get().to(orders::index)))
    .service(web::resource("/orders/{id}/activity").route(web::get().to(orders::activity)))
    .service(web::resource("/orders/{id}/details").route(web::get().to(orders::details)))
    .service(web::resource("/orders/{id}/refund").route(web::patch().to(orders::refund)))
    .service(web::resource("/orders/{id}/resend_confirmation").route(web::post().to(orders::resend_confirmation)))
    .service(
        web::resource("/orders/{id}/send_box_office_instructions")
            .route(web::post().to(orders::send_box_office_instructions)),
    )
    .service(web::resource("/orders/{id}/tickets").route(web::get().to(orders::tickets)))
    .service(web::resource("/orders/{id}/transfers").route(web::get().to(transfers::index)))
    .service(web::resource("/orders/{id}").route(web::get().to(orders::show)))
    .service(
        web::resource("/organization_venues/{id}")
            .route(web::get().to(organization_venues::show))
            .route(web::delete().to(organization_venues::destroy)),
    )
    .service(
        web::resource("/organizations/{id}/artists")
            .route(web::get().to(artists::show_from_organizations))
            .route(web::post().to(organizations::add_artist)),
    )
    .service(web::resource("/organizations/{id}/events").route(web::get().to(events::show_from_organizations)))
    .service(web::resource("/organizations/{id}/export_event_data").route(web::get().to(events::export_event_data)))
    .service(
        web::resource("/organizations/{id}/fans/{user_id}/activity")
            .wrap(CacheResourceTransform::new(CacheUsersBy::OrganizationScopePresence(
                OrganizationLoad::Path,
                Scopes::OrgFans,
            )))
            .route(web::get().to(users::activity)),
    )
    .service(
        web::resource("/organizations/{id}/fans/{user_id}/history")
            .wrap(CacheResourceTransform::new(CacheUsersBy::OrganizationScopePresence(
                OrganizationLoad::Path,
                Scopes::OrgFans,
            )))
            .route(web::get().to(users::history)),
    )
    .service(
        web::resource("/organizations/{id}/fans/{user_id}")
            .wrap(CacheResourceTransform::new(CacheUsersBy::OrganizationScopePresence(
                OrganizationLoad::Path,
                Scopes::OrgFans,
            )))
            .route(web::get().to(users::profile)),
    )
    .service(
        web::resource("/organizations/{id}/fee_schedule")
            .route(web::get().to(organizations::show_fee_schedule))
            .route(web::post().to(organizations::add_fee_schedule)),
    )
    .service(
        web::resource("/organizations/{id}/fans")
            .wrap(CacheResourceTransform::new(CacheUsersBy::OrganizationScopePresence(
                OrganizationLoad::Path,
                Scopes::OrgFans,
            )))
            .route(web::get().to(organizations::search_fans)),
    )
    .service(
        web::resource("/organizations/{id}/invites/{invite_id}").route(web::delete().to(organization_invites::destroy)),
    )
    .service(
        web::resource("/organizations/{id}/organization_venues")
            .route(web::get().to(organization_venues::organizations_index))
            .route(web::post().to(organization_venues::create)),
    )
    .service(
        web::resource("/organizations/{id}/settlements")
            .route(web::get().to(settlements::index))
            .route(web::post().to(settlements::create)),
    )
    .service(
        web::resource("/organizations/{id}/invites")
            .route(web::get().to(organization_invites::index))
            .route(web::post().to(organization_invites::create)),
    )
    .service(
        web::resource("/organizations/{id}/users")
            .route(web::post().to(organizations::add_or_replace_user))
            .route(web::put().to(organizations::add_or_replace_user))
            .route(web::get().to(organizations::list_organization_members)),
    )
    .service(web::resource("/organizations/{id}/users/{user_id}").route(web::delete().to(organizations::remove_user)))
    .service(web::resource("/organizations/{id}/venues").route(web::get().to(venues::show_from_organizations)))
    .service(
        web::resource("/organizations/{id}")
            .route(web::get().to(organizations::show))
            .route(web::patch().to(organizations::update)),
    )
    .service(
        web::resource("/organizations")
            .route(web::get().to(organizations::index))
            .route(web::post().to(organizations::create)),
    )
    .service(
        web::resource("/password_reset")
            .route(web::post().to(password_resets::create))
            .route(web::put().to(password_resets::update)),
    )
    .service(web::resource("/payments/callback/{nonce}/{id}").route(web::get().to(payments::callback)))
    .service(web::resource("/payment_methods").route(web::get().to(payment_methods::index)))
    .service(web::resource("/redemption_codes/{code}").route(web::get().to(redemption_codes::show)))
    .service(
        web::resource("/regions/{id}")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(regions::show))
            .route(web::put().to(regions::update)),
    )
    .service(
        web::resource("/regions")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(regions::index))
            .route(web::post().to(regions::create)),
    )
    .service(web::resource("/reports/{id}").route(web::get().to(reports::get_report)))
    .service(web::resource("/slugs").route(web::get().to(slugs::index)))
    .service(
        web::resource("/slugs/{id}")
            .route(web::get().to(slugs::show))
            .route(web::put().to(slugs::update)),
    )
    .service(web::resource("/status").route(web::get().to(status::check)))
    .service(
        web::resource("/stages/{id}")
            .route(web::get().to(stages::show))
            .route(web::put().to(stages::update))
            .route(web::delete().to(stages::delete)),
    )
    .service(web::resource("/settlement_adjustments/{id}").route(web::delete().to(settlement_adjustments::destroy)))
    .service(
        web::resource("/settlements/{id}/adjustments")
            .route(web::get().to(settlement_adjustments::index))
            .route(web::post().to(settlement_adjustments::create)),
    )
    .service(
        web::resource("/settlements/{id}")
            .route(web::get().to(settlements::show))
            .route(web::delete().to(settlements::destroy)),
    )
    .service(web::resource("/tickets/transfer").route(web::post().to(tickets::transfer_authorization)))
    .service(web::resource("/tickets/receive").route(web::post().to(tickets::receive_transfer)))
    .service(web::resource("/tickets/send").route(web::post().to(tickets::send_via_email_or_phone)))
    .service(
        web::resource("/tickets/{id}")
            .route(web::get().to(tickets::show))
            .route(web::patch().to(tickets::update)),
    )
    .service(web::resource("/tickets").route(web::get().to(tickets::index)))
    .service(web::resource("/tickets/{id}/redeem").route(web::get().to(tickets::show_redeemable_ticket)))
    .service(web::resource("/transfers/transfer_key/{id}").route(web::get().to(transfers::show_by_transfer_key)))
    .service(web::resource("/transfers/activity").route(web::get().to(transfers::activity)))
    .service(web::resource("/transfers/{id}").route(web::delete().to(transfers::cancel)))
    .service(web::resource("/transfers").route(web::get().to(transfers::index)))
    .service(
        web::resource("/users/me")
            .route(web::get().to(users::current_user))
            .route(web::put().to(users::update_current_user)),
    )
    .service(web::resource("/users/register").route(web::post().to(users::register)))
    .service(web::resource("/users/{id}/tokens").route(web::get().to(users::show_push_notification_tokens_for_user_id)))
    .service(
        web::resource("/users/tokens")
            .route(web::get().to(users::show_push_notification_tokens))
            .route(web::post().to(users::add_push_notification_token)),
    )
    .service(web::resource("/users/tokens/{id}").route(web::delete().to(users::remove_push_notification_token)))
    .service(web::resource("/users").route(web::post().to(users::register_and_login)))
    .service(
        web::resource("/users/{id}")
            .route(web::get().to(users::show))
            .route(web::delete().to(users::delete)),
    )
    .service(web::resource("/user_invites").route(web::post().to(user_invites::create)))
    .service(web::resource("/users/{id}/organizations").route(web::get().to(users::list_organizations)))
    .service(
        web::resource("/venues/{id}/organization_venues")
            .route(web::get().to(organization_venues::venues_index))
            .route(web::post().to(organization_venues::create)),
    )
    .service(
        web::resource("/venues/{id}/stages")
            .route(web::post().to(stages::create))
            .route(web::get().to(stages::index)),
    )
    .service(web::resource("/venues/{id}/toggle_privacy").route(web::put().to(venues::toggle_privacy)))
    .service(
        web::resource("/venues/{id}")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(venues::show))
            .route(web::put().to(venues::update)),
    )
    .service(
        web::resource("/venues")
            .wrap(CacheResourceTransform::new(CacheUsersBy::AnonymousOnly))
            .route(web::get().to(venues::index))
            .route(web::post().to(venues::create)),
    )
    .service(
        web::resource("/sitemap.xml")
            .wrap(CacheResourceTransform::new(CacheUsersBy::None))
            .route(web::get().to(sitemap_gen::index)),
    );
}
