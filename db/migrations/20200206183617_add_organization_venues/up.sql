CREATE TABLE organization_venues
(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    organization_id UUID NOT NULL REFERENCES organizations(id),
    venue_id UUID NOT NULL REFERENCES venues(id),
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX index_organization_venues_organization_id_venue_id ON organization_venues (organization_id, venue_id);
CREATE INDEX index_organization_venues_venue_id ON organization_venues (venue_id);

INSERT INTO organization_venues(organization_id, venue_id)
SELECT v.organization_id, v.id FROM venues v WHERE v.organization_id IS NOT NULL;

ALTER TABLE venues DROP organization_id;
