
create table rarities (
    id UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    event_id uuid null REFERENCES events (id),
    name text not null,
    rank int not null,
    color text null,
    created_at timestamp not null default now(),
    updated_at timestamp not null default now()
);

-- create table loot_boxes(
--   id uuid primary key not null default gen_random_uuid(),
--   promo_image_url text null,
--   name text not null,
--   price_in_cents bigint not null,
--   description text,
--   rank int not null default 0,
--   created_at TIMESTAMP not null default now(),
--   updated_at timestamp not null default now()
-- );
alter table ticket_types
    add rarity_id uuid null references rarities (id);

alter table ticket_types
    add ticket_type_type varchar(20) not null default 'Token'; -- I know. Will rename later

alter table ticket_types
    add promo_image_url text null;

alter table ticket_types
    add content_url text null;


create table loot_box_contents (
  id uuid primary key not null default gen_random_uuid(),
  ticket_type_id uuid not null REFERENCES ticket_types (id),
  content_event_id uuid not null references events (id),  -- little bit weird here
  min_rarity_id uuid  null references rarities (id),
  max_rarity_id uuid null references rarities (id),
  content_ticket_type_id uuid null references ticket_types (id),
  quantity_per_box int not null,
  created_at timestamp not null default now(),
  updated_at timestamp not NULL default now()
);

-- create table loot_box_instances(
--   id uuid primary key not null default gen_random_uuid(),
--     loot_box_id uuid not null references loot_boxes (id),
--     order_item_id uuid references order_items (id),
--     wallet_id uuid references wallets (id) not null,
--     reserved_until timestamp,
--     status text not null,
--     opened_at timestamp null,
--     created_at timestamp not null,
--     updated_at timestamp not null
-- );

alter table ticket_instances
    add parent_id   uuid REFERENCES ticket_instances (id);





-- alter table order_items
--   add loot_box_id uuid references loot_boxes (id)
--     ;



