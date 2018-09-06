use chrono::prelude::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use models::{Organization, OrganizationInvite, User};
use uuid::Uuid;

#[allow(dead_code)]
pub struct OrgInviteBuilder<'a> {
    organization_id: Option<Uuid>,
    invitee_id: Option<Uuid>,
    user_email: String,
    create_at: NaiveDateTime,
    security_token: Option<Uuid>,
    user_id: Option<Uuid>,
    status_change_at: Option<NaiveDateTime>,
    accepted: Option<i16>,
    connection: &'a PgConnection,
}

impl<'a> OrgInviteBuilder<'a> {
    pub fn new(connection: &PgConnection) -> OrgInviteBuilder {
        OrgInviteBuilder {
            organization_id: None,
            invitee_id: None,
            user_email: "test@test.com".into(),
            create_at: NaiveDate::from_ymd(2016, 7, 8).and_hms(9, 10, 11),
            security_token: Some(Uuid::new_v4()),
            connection,
            user_id: None,
            status_change_at: None,
            accepted: None,
        }
    }

    pub fn with_org(mut self, org: &Organization) -> OrgInviteBuilder<'a> {
        self.organization_id = Some(org.id.clone());
        self
    }

    pub fn with_invitee(mut self, invitee: &User) -> OrgInviteBuilder<'a> {
        self.invitee_id = Some(invitee.id.clone());
        self
    }

    pub fn with_email(mut self, email: &String) -> OrgInviteBuilder<'a> {
        self.user_email = email.clone();
        self
    }

    pub fn link_to_user(mut self, user: &User) -> OrgInviteBuilder<'a> {
        self.user_id = Some(user.id.clone());
        self
    }

    pub fn with_security_token(mut self, security_token: Option<Uuid>) -> Self {
        self.security_token = security_token;
        self
    }

    pub fn finish(&self) -> OrganizationInvite {
        let orginvite = OrganizationInvite::create(
            self.organization_id.unwrap(),
            self.invitee_id.unwrap(),
            &self.user_email,
            self.user_id,
        ).commit(self.connection)
            .unwrap();
        orginvite
    }
}
