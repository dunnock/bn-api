use chrono::NaiveDateTime;
use diesel;
use diesel::dsl;
use diesel::prelude::*;
use models::enums::Roles;
use models::{EventUser, Organization, Scopes, User};
use schema::{event_users, events, organization_users};
use serde_json;
use serde_json::Value;
use utils::errors::DatabaseError;
use utils::errors::ErrorCode;
use utils::errors::*;
use uuid::Uuid;

#[derive(AsChangeset, Associations, Identifiable, Queryable, Serialize)]
#[belongs_to(User)]
#[belongs_to(Organization)]
#[table_name = "organization_users"]
pub struct OrganizationUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub role: Vec<Roles>,
    pub additional_scopes: Option<serde_json::Value>,
}

#[derive(Default, Clone, Debug, Serialize, PartialEq)]
pub struct AdditionalOrgMemberScopes {
    pub additional: Vec<Scopes>,
    pub revoked: Vec<Scopes>,
}

impl From<serde_json::Value> for AdditionalOrgMemberScopes {
    fn from(json_value: Value) -> Self {
        let parse = |json_value: &Value| -> Vec<Scopes> {
            json_value
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|s| s.as_str().map(|g| g.parse::<Scopes>()))
                        .filter(|s| {
                            if let Some(res) = s {
                                return res.is_ok();
                            }
                            return false;
                        })
                        .map(|s| s.unwrap().unwrap())
                        .collect::<Vec<Scopes>>()
                })
                .unwrap_or(vec![])
        };
        let additional = parse(&json_value["additional"]);
        let revoked = parse(&json_value["revoked"]);
        AdditionalOrgMemberScopes { additional, revoked }
    }
}

#[derive(Insertable)]
#[table_name = "organization_users"]
pub struct NewOrganizationUser {
    pub organization_id: Uuid,
    pub user_id: Uuid,
    role: Vec<Roles>,
}

impl NewOrganizationUser {
    pub fn is_event_user(&self) -> bool {
        OrganizationUser::contains_role_for_event_user(&self.role)
    }

    pub fn commit(self, conn: &PgConnection) -> Result<OrganizationUser, DatabaseError> {
        let existing_user = OrganizationUser::find_by_user_id(self.user_id, self.organization_id, conn).optional()?;
        match existing_user {
            Some(mut organization_user) => {
                // If the new role is a promoter role, combine with existing roles
                if self.is_event_user() {
                    if organization_user.is_event_user() {
                        // Merge roles
                        organization_user.role.extend(self.role);
                        organization_user.role.sort();
                        organization_user.role.dedup();
                    } else {
                        // User is getting updated to an event only role, replace existing
                        organization_user.role = self.role;
                    }
                } else {
                    // Replace roles, remove existing event user links as other roles are cross all events
                    EventUser::destroy_all(self.user_id, conn)?;
                    organization_user.role = self.role;
                }

                diesel::update(organization_users::table.filter(organization_users::id.eq(organization_user.id)))
                    .set((organization_users::role.eq(organization_user.role),))
                    .get_result(conn)
                    .to_db_error(
                        ErrorCode::UpdateError,
                        "Could not add role to existing organization user",
                    )
            }
            None => DatabaseError::wrap(
                ErrorCode::InsertError,
                "Could not create new organization user",
                diesel::insert_into(organization_users::table)
                    .values(self)
                    .get_result(conn),
            ),
        }
    }
}

impl OrganizationUser {
    pub fn is_event_user(&self) -> bool {
        OrganizationUser::contains_role_for_event_user(&self.role)
    }

    pub(crate) fn contains_role_for_event_user(role: &[Roles]) -> bool {
        role.contains(&Roles::Promoter) || role.contains(&Roles::PromoterReadOnly)
    }

    pub fn create(organization_id: Uuid, user_id: Uuid, role: Vec<Roles>) -> NewOrganizationUser {
        NewOrganizationUser {
            organization_id,
            user_id,
            role,
        }
    }

    pub fn event_ids(&self, conn: &PgConnection) -> Result<Vec<Uuid>, DatabaseError> {
        organization_users::table
            .inner_join(events::table.on(events::organization_id.eq(organization_users::organization_id)))
            .inner_join(
                event_users::table.on(event_users::user_id
                    .eq(self.user_id)
                    .and(event_users::event_id.eq(events::id))),
            )
            .filter(organization_users::organization_id.eq(self.organization_id))
            .select(event_users::event_id)
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organization user event ids")
    }

    // This will overwrite the additional_scopes field to whatever you set it to here.
    // If you want to maintain previous scopes you'll need to pull them in your function and send them back
    pub fn set_additional_scopes(
        &self,
        additional_scopes: AdditionalOrgMemberScopes,
        conn: &PgConnection,
    ) -> Result<OrganizationUser, DatabaseError> {
        //Clean up the additional_scopes
        let mut additional = additional_scopes.additional.clone();
        additional.sort();
        additional.dedup();

        let mut revoked = additional_scopes.revoked.clone();
        revoked.sort();
        revoked.dedup();

        let additional_scopes = AdditionalOrgMemberScopes { additional, revoked };

        diesel::update(organization_users::table.filter(organization_users::id.eq(self.id)))
            .set((
                organization_users::additional_scopes.eq(json!(additional_scopes)),
                organization_users::updated_at.eq(dsl::now),
            ))
            .get_result(conn)
            .to_db_error(ErrorCode::UpdateError, "Could not update additional_scopes")
    }

    pub fn find_users_by_organization(
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<Vec<OrganizationUser>, DatabaseError> {
        organization_users::table
            .filter(organization_users::organization_id.eq(organization_id))
            .load(conn)
            .to_db_error(ErrorCode::QueryError, "Could not load organization users")
    }

    pub fn find_by_user_id(
        user_id: Uuid,
        organization_id: Uuid,
        conn: &PgConnection,
    ) -> Result<OrganizationUser, DatabaseError> {
        organization_users::table
            .filter(
                organization_users::user_id
                    .eq(user_id)
                    .and(organization_users::organization_id.eq(organization_id)),
            )
            .first(conn)
            .to_db_error(ErrorCode::QueryError, "Could not find user for organization")
    }
}
