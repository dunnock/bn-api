drop index index_ticket_instances_listing_id;

alter table ticket_instances
    drop listing_id;

drop index index_listings_user_id;

drop table listings;