create table marketplace_accounts(
    id  uuid primary key not null default gen_random_uuid(),
    user_id uuid not null references users(id),
    status text not null,
    marketplace_id text null,
    marketplace_user_id text not null,
    marketplace_password text not null,
    deleted_at timestamp,
    created_at timestamp not null,
    updated_at timestamp not null
);

create index index_marketplace_account_user_id on marketplace_accounts (user_id);