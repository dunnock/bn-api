ALTER TABLE venues ADD organization_id UUID references organizations(id);

UPDATE venues SET organization_id = ov.organization_id
FROM organization_venues ov
JOIN venues v ON ov.venue_id = v.id
WHERE venues.id = v.id;

DROP INDEX IF EXISTS index_organization_venues_venue_id;
DROP INDEX IF EXISTS index_organization_venues_organization_id_venue_id;
DROP TABLE IF EXISTS organization_venues;
