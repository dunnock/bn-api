create table listings (
    id uuid not null primary key default gen_random_uuid(),
    title text not null,
    user_id uuid not null references users (id),
    marketplace_id text null,
    asking_price_in_cents bigint not null,
    status text not null,
    created_at timestamp not null,
    updated_at timestamp not null,
    deleted_at timestamp null
);

create index index_listings_user_id on listings(user_id);

alter table ticket_instances
    add listing_id uuid null references listings( id);

create index index_ticket_instances_listing_id on ticket_instances (listing_id)

