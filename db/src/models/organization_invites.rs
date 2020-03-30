use chrono::{Duration, NaiveDateTime, Utc};
use diesel;
use diesel::dsl::sql;
use diesel::pg::types::sql_types::Array;
use diesel::prelude::*;
use diesel::sql_types::{Text, Timestamp, Uuid as dUuid};
use models::*;
use schema::{organization_invites, organizations, users};
use std::borrow::Cow;
use utils::errors::{ConvertToDatabaseError, DatabaseError, ErrorCode, Optional};
use uuid::Uuid;
use validator::Validate;
use validators::{self, *};

pub const INVITE_EXPIRATION_PERIOD_IN_DAYS: i64 = 7;

#[derive(
    Associations,
    Insertable,
    Queryable,
    Identifiable,
    PartialEq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    AsChangeset,
    QueryableByName,
)]
#[belongs_to(Organization, foreign_key = "organization_id")]
#[belongs_to(User, foreign_key = "inviter_id")]
#[table_name = "organization_invites"]
pub struct OrganizationInvite {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub inviter_id: Uuid,
    pub user_email: String,
    pub created_at: NaiveDateTime,
    pub security_token: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub accepted: Option<i16>,
    pub updated_at: NaiveDateTime,
    pub sent_invite: bool,
    pub roles: Vec<Roles>,
    pub event_ids: Vec<Uuid>,
}

#[derive(Insertable, PartialEq, Debug, Deserialize, Validate)]
#[table_name = "organization_invites"]
pub struct NewOrganizationInvite {
    pub organization_id: Uuid,
    pub inviter_id: Uuid,
    #[validate(email(message = "User email is invalid"))]
    pub user_email: String,
    pub security_token: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub roles: Vec<Roles>,
    pub event_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, PartialEq, Queryable, Serialize, QueryableByName)]
pub struct DisplayInvite {
    #[sql_type = "dUuid"]
    pub id: Uuid,
    #[sql_type = "Text"]
    pub organization_name: String,
    #[sql_type = "Text"]
    pub inviter_name: String,
    #[sql_type = "Timestamp"]
    pub expires_at: NaiveDateTime,
}

impl NewOrganizationInvite {
    pub fn validate_record(&self, conn: &PgConnection) -> Result<(), DatabaseError> {
        let mut validation_errors = validators::append_validation_error(
            self.validate(),
            "event_ids",
            event_ids_belong_to_organization_validation(
                true,
                self.organization_id,
                &self.event_ids.clone().unwrap_or(Vec::new()),
                conn,
            )?,
        );

        if let Some(user_id) = self.user_id {
            let organization_user =
                OrganizationUser::find_by_user_id(user_id, self.organization_id, conn).optional()?;
            if organization_user.is_some() && !organization_user.unwrap().is_event_user() {
                let mut validation_error =
                    create_validation_error("uniqueness", "User already belongs to organization");
                validation_error.add_param(Cow::from("user_id"), &user_id);
                validation_errors =
                    validators::append_validation_error(validation_errors, "user_id", Err(validation_error));
            }
        }

        Ok(validation_errors?)
    }

    pub fn commit(&mut self, conn: &PgConnection) -> Result<OrganizationInvite, DatabaseError> {
        // Clear event ids if role is not the limited access role
        if Roles::get_event_limited_roles()
            .iter()
            .find(|r| self.roles.contains(&r))
            .is_none()
            || self.event_ids.is_none()
        {
            self.event_ids = Some(Vec::new());
        }
        self.user_email = self.user_email.to_lowercase();
        self.security_token = Some(Uuid::new_v4());
        self.validate_record(conn)?;
        let res = diesel::insert_into(organization_invites::table)
            .values(&*self)
            .get_result(conn);
        DatabaseError::wrap(ErrorCode::InsertError, "Could not create new invite", res)
    }
}

impl OrganizationInvite {
    pub fn is_event_user(&self) -> bool {
        OrganizationUser::contains_role_for_event_user(&self.roles)
    }

    pub fn create(
        org_id: Uuid,
        inviter_id: Uuid,
        email: &str,
        user_id: Option<Uuid>,
        roles: Vec<Roles>,
        event_ids: Option<Vec<Uuid>>,
    ) -> NewOrganizationInvite {
        NewOrganizationInvite {
            organization_id: org_id,
            inviter_id,
            user_email: email.into(),
            security_token: None,
            user_id,
            roles,
            event_ids,
        }
    }

    pub fn organization(&self, conn: &PgConnection) -> Result<Organization, DatabaseError> {
        Organization::find(self.organization_id, conn)
    }

    pub fn change_invite_status(&mut self, change_status: i16, conn: &PgConnection) -> Result<(), DatabaseError> {
        self.security_token = None;
        self.accepted = Some(change_status);
        self.updated_at = Utc::now().naive_utc();
        diesel::update(organization_invites::table.filter(organization_invites::id.eq(self.id)))
            .set((
                organization_invites::security_token.eq(self.security_token),
                organization_invites::accepted.eq(self.accepted),
                organization_invites::updated_at.eq(self.updated_at),
            ))
            .execute(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update organization invite table")?;

        Ok(())
    }

    pub fn accept_invite(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        self.change_invite_status(1, conn)
    }

    pub fn decline_invite(&mut self, conn: &PgConnection) -> Result<(), DatabaseError> {
        self.change_invite_status(0, conn)
    }

    pub fn get_invite_display(token: &Uuid, conn: &PgConnection) -> Result<DisplayInvite, DatabaseError> {
        let expiry_date = Utc::now().naive_utc() - Duration::days(INVITE_EXPIRATION_PERIOD_IN_DAYS);

        organization_invites::table
            .inner_join(users::table.on(users::id.eq(organization_invites::inviter_id)))
            .inner_join(organizations::table.on(organizations::id.eq(organization_invites::organization_id)))
            .filter(organization_invites::accepted.is_null())
            .filter(organization_invites::security_token.eq(token))
            .filter(organization_invites::created_at.gt(expiry_date))
            .select((
                organization_invites::id,
                organizations::name,
                sql("CONCAT(users.first_name, ' ',  users.last_name) AS inviter_name"),
                sql::<Timestamp>("organization_invites.created_at + (INTERVAL '1' day) * ")
                    .bind::<diesel::sql_types::BigInt, _>(INVITE_EXPIRATION_PERIOD_IN_DAYS),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::QueryError, "Cannot find organization invite")
    }

    pub fn find_by_token(token: Uuid, conn: &PgConnection) -> Result<OrganizationInvite, DatabaseError> {
        let expiry_date = Utc::now().naive_utc() - Duration::days(INVITE_EXPIRATION_PERIOD_IN_DAYS);
        DatabaseError::wrap(
            ErrorCode::AccessError,
            "No valid token found",
            organization_invites::table
                .filter(organization_invites::accepted.is_null())
                .filter(organization_invites::security_token.eq(token))
                .filter(organization_invites::created_at.gt(expiry_date))
                .get_result(conn),
        )
    }

    pub fn find_active_organization_invite_for_email(
        email: &String,
        organization: &Organization,
        event_ids: Option<&[Uuid]>,
        conn: &PgConnection,
    ) -> Result<Option<OrganizationInvite>, DatabaseError> {
        let mut query = organization_invites::table
            .filter(organization_invites::user_email.eq(email))
            .filter(organization_invites::security_token.is_not_null())
            .filter(organization_invites::organization_id.eq(organization.id))
            .into_boxed();

        if let Some(event_ids) = event_ids {
            query = query.filter(sql("organization_invites.event_ids && ").bind::<Array<dUuid>, _>(event_ids.clone()));
        }

        query
            .first::<OrganizationInvite>(conn)
            .optional()
            .to_db_error(ErrorCode::QueryError, "Cannot find organization invite")
    }

    pub fn find_all_active_organization_invites_by_email(
        email: &String,
        organization: &Organization,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationInvite>, DatabaseError> {
        organization_invites::table
            .filter(organization_invites::user_email.eq(email))
            .filter(organization_invites::security_token.is_not_null())
            .filter(organization_invites::organization_id.eq(organization.id))
            .load::<OrganizationInvite>(conn)
            .to_db_error(ErrorCode::QueryError, "Cannot find organization invite")
    }

    pub fn change_sent_status(
        &self,
        sent_status: bool,
        conn: &PgConnection,
    ) -> Result<OrganizationInvite, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::UpdateError,
            "Could not update region",
            diesel::update(self)
                .set(organization_invites::sent_invite.eq(sent_status))
                .get_result(conn),
        )
    }

    pub fn find(id: Uuid, conn: &PgConnection) -> Result<OrganizationInvite, DatabaseError> {
        DatabaseError::wrap(
            ErrorCode::QueryError,
            "Error loading organization invite",
            organization_invites::table.find(id).first(conn),
        )
    }

    pub fn find_pending_by_organization_paged(
        organization_id: Uuid,
        page: u32,
        limit: u32,
        conn: &PgConnection,
    ) -> Result<Payload<DisplayInvite>, DatabaseError> {
        let total: i64 = organization_invites::table
            .filter(organization_invites::organization_id.eq(organization_id))
            .filter(organization_invites::accepted.is_null())
            .count()
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not get total invites for organization")?;

        let paging = Paging::new(page, limit);
        let mut payload = Payload::new(
            organization_invites::table
                .inner_join(users::table.on(users::id.eq(organization_invites::inviter_id)))
                .inner_join(organizations::table.on(organizations::id.eq(organization_invites::organization_id)))
                .filter(organization_invites::organization_id.eq(organization_id))
                .filter(organization_invites::accepted.is_null())
                .order_by(organization_invites::user_email.asc())
                .limit(limit as i64)
                .offset((page * limit) as i64)
                .select((
                    organization_invites::id,
                    organizations::name,
                    sql("CONCAT(users.first_name, ' ',  users.last_name) AS inviter_name"),
                    sql::<Timestamp>("organization_invites.created_at + (INTERVAL '1' day) * ")
                        .bind::<diesel::sql_types::BigInt, _>(INVITE_EXPIRATION_PERIOD_IN_DAYS),
                ))
                .load(conn)
                .to_db_error(ErrorCode::QueryError, "Could not retrieve invites for organization")?,
            paging,
        );

        // TODO: remove this when other structs implement paging
        payload.paging.total = total as u64;
        payload.paging.page = page;
        payload.paging.limit = limit;
        Ok(payload)
    }

    pub fn destroy(&self, conn: &PgConnection) -> Result<usize, DatabaseError> {
        if let Some(accepted) = self.accepted {
            return Err(DatabaseError::new(
                ErrorCode::BusinessProcessError,
                Some(
                    format!(
                        "Cannot destroy invite it has already been {}.",
                        if accepted == 1 { "accepted" } else { "declined" }
                    )
                    .to_string(),
                ),
            ));
        }

        DatabaseError::wrap(
            ErrorCode::DeleteError,
            "Failed to destroy organization invite",
            diesel::delete(self).execute(conn),
        )
    }

    pub fn find_pending_by_organization(
        organization_id: Uuid,
        event_id: Option<Uuid>,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationInvite>, DatabaseError> {
        let mut query = organization_invites::table
            .filter(organization_invites::organization_id.eq(organization_id))
            .filter(organization_invites::accepted.is_null())
            .into_boxed();

        query = match event_id {
            Some(id) => query.filter(organization_invites::event_ids.contains(vec![id])),
            None => query.filter(organization_invites::event_ids.eq(Vec::<Uuid>::new())),
        };
        query
            .order_by(organization_invites::user_email.asc())
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load invites for organization")
    }
}
