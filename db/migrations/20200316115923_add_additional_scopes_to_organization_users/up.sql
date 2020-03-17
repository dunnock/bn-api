ALTER TABLE organization_users
    -- the additional_scopes is a json object of {"additional": ["ScopeEnum"], "revoked": ["ScopeEnum"]}
    ADD COLUMN additional_scopes JSONB NULL;