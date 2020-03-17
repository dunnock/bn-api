CREATE TABLE announcements
(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    message VARCHAR(190) NOT NULL,
    organization_id uuid NULL references organizations(id),
    deleted_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX index_announcements_organization_id ON announcements (organization_id);

CREATE TABLE announcement_engagements
(
    id UUID PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL references users(id),
    announcement_id uuid NOT NULL references announcements(id),
    action TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX index_announcement_engagements_user_id ON announcement_engagements (user_id);
CREATE UNIQUE INDEX index_announcement_engagements_announcement_id_user_id ON announcement_engagements (announcement_id, user_id);
