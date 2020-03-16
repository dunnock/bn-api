use crate::errors::*;
use crate::extractors::OptionalUser;
use actix_web::{HttpRequest, Result};
use bigneon_db::models::User as DbUser;
use bigneon_db::models::{scopes, Event, EventUser, Order, Organization, Roles, Scopes};
use bigneon_db::prelude::errors::EnumParseError;
use bigneon_db::prelude::Optional;
use diesel::PgConnection;
use log::Level::Warn;
use logging::*;
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

const MISSING_PERMISSIONS_MESSAGING: &str = "User does not have the required permissions";

#[derive(Clone, Debug)]
pub struct User {
    pub user: DbUser,
    pub global_scopes: Vec<String>,
    pub ip_address: Option<String>,
    pub uri: String,
    pub method: String,
    pub global_scopes_only: bool,
}

impl User {
    pub fn new(
        user: DbUser,
        request: &HttpRequest,
        limited_scopes: Option<Vec<String>>,
    ) -> Result<User, EnumParseError> {
        let mut result = User {
            user: user.clone(),
            global_scopes: vec![],
            ip_address: request.connection_info().remote().map(|i| i.to_string()),
            uri: request.uri().to_string(),
            method: request.method().to_string(),
            global_scopes_only: false,
        };
        if let Some(scopes) = limited_scopes {
            result.global_scopes = scopes;
            result.global_scopes_only = true;
        } else {
            let global_scopes = user.get_global_scopes().into_iter().map(|s| s.to_string()).collect();
            result.global_scopes = global_scopes;
        }
        Ok(result)
    }

    pub fn id(&self) -> Uuid {
        self.user.id
    }

    pub fn email(&self) -> Option<String> {
        self.user.email.clone()
    }

    fn check_scope_access(
        &self,
        scope: Scopes,
        organization: Option<&Organization>,
        event_id: Option<Uuid>,
        connection: Option<&PgConnection>,
        log_on_failure: bool,
    ) -> Result<bool, BigNeonError> {
        if self.global_scopes_only {
            if self.global_scopes.contains(&scope.to_string()) {
                return Ok(true);
            } else {
                return Ok(false);
            }
        }

        if self.global_scopes.contains(&scope.to_string()) {
            return Ok(true);
        }

        let mut logging_data = HashMap::new();

        if let (Some(organization), Some(connection)) = (organization, connection) {
            let organization_scopes = organization.get_scopes_for_user(&self.user, connection)?;
            logging_data.insert("organization_scopes", json!(organization_scopes));
            logging_data.insert("organization_id", json!(organization.id));

            if let Some(event_id) = event_id {
                // If the user's roles include an event limited role
                let user_roles = organization.get_roles_for_user(&self.user, connection)?;
                if Roles::get_event_limited_roles()
                    .iter()
                    .find(|r| user_roles.contains(&r))
                    .is_some()
                {
                    let event_user = EventUser::find_by_event_id_user_id(event_id, self.id(), connection).optional()?;
                    if let Some(event_user) = event_user {
                        let scopes = scopes::get_scopes(vec![event_user.role]);

                        if scopes.contains(&scope) {
                            return Ok(true);
                        }
                    }
                } else if organization_scopes.contains(&scope) {
                    return Ok(true);
                }
            } else if organization_scopes.contains(&scope) {
                return Ok(true);
            }
        }

        logging_data.insert("accessed_scope", json!(scope.to_string()));
        logging_data.insert("global_scopes", json!(self.global_scopes));

        if log_on_failure {
            self.log_unauthorized_access_attempt(logging_data);
        }
        Ok(false)
    }

    pub fn has_scope(&self, scope: Scopes) -> Result<bool, BigNeonError> {
        self.check_scope_access(scope, None, None, None, false)
    }

    pub fn has_scope_for_organization_event(
        &self,
        scope: Scopes,
        organization: &Organization,
        event_id: Uuid,
        conn: &PgConnection,
    ) -> Result<bool, BigNeonError> {
        self.check_scope_access(scope, Some(organization), Some(event_id), Some(conn), false)
    }

    pub fn has_scope_for_order(&self, scope: Scopes, order: &Order, conn: &PgConnection) -> Result<bool, BigNeonError> {
        let mut has_scope = false;
        for event in order.events(conn)? {
            if self.check_scope_access(
                scope,
                Some(&event.organization(conn)?),
                Some(event.id),
                Some(conn),
                false,
            )? {
                has_scope = true;
            }
        }
        Ok(has_scope)
    }

    pub fn has_scope_for_organization(
        &self,
        scope: Scopes,
        organization: &Organization,
        conn: &PgConnection,
    ) -> Result<bool, BigNeonError> {
        self.check_scope_access(scope, Some(organization), None, Some(conn), false)
    }

    pub fn log_unauthorized_access_attempt(&self, mut logging_data: HashMap<&'static str, Value>) {
        logging_data.insert("user_id", json!(self.id()));
        logging_data.insert("user_name", json!(self.user.full_name()));
        logging_data.insert("ip_address", json!(self.ip_address));
        logging_data.insert("url", json!(self.uri));
        logging_data.insert("method", json!(self.method));
        jlog!(Warn, "Unauthorized access attempt", logging_data);
    }

    pub fn requires_scope(&self, scope: Scopes) -> Result<(), BigNeonError> {
        if self.check_scope_access(scope, None, None, None, true)? {
            return Ok(());
        }
        Err(AuthError::new(AuthErrorType::Unauthorized, MISSING_PERMISSIONS_MESSAGING.to_string()).into())
    }

    pub fn requires_scope_for_order(
        &self,
        scope: Scopes,
        order: &Order,
        conn: &PgConnection,
    ) -> Result<(), BigNeonError> {
        if !self.has_scope_for_order(scope, order, conn)? {
            let mut logging_data = HashMap::new();
            logging_data.insert("accessed_scope", json!(scope.to_string()));
            logging_data.insert("global_scopes", json!(self.global_scopes));
            logging_data.insert("order_id", json!(order.id));
            self.log_unauthorized_access_attempt(logging_data);

            return Err(AuthError::new(AuthErrorType::Unauthorized, MISSING_PERMISSIONS_MESSAGING.to_string()).into());
        }
        Ok(())
    }

    pub fn requires_scope_for_organization_event(
        &self,
        scope: Scopes,
        organization: &Organization,
        event: &Event,
        conn: &PgConnection,
    ) -> Result<(), BigNeonError> {
        if self.check_scope_access(scope, Some(organization), Some(event.id), Some(conn), true)? {
            return Ok(());
        }
        Err(AuthError::new(AuthErrorType::Unauthorized, MISSING_PERMISSIONS_MESSAGING.to_string()).into())
    }

    pub fn requires_scope_for_organization(
        &self,
        scope: Scopes,
        organization: &Organization,
        conn: &PgConnection,
    ) -> Result<(), BigNeonError> {
        if self.check_scope_access(scope, Some(organization), None, Some(conn), true)? {
            return Ok(());
        }
        Err(AuthError::new(AuthErrorType::Unauthorized, MISSING_PERMISSIONS_MESSAGING.to_string()).into())
    }

    pub fn into_optional(self) -> OptionalUser {
        OptionalUser(Some(self))
    }
}
