use crate::db::Connection;
use crate::extractors::*;
use crate::helpers::*;
use crate::server::AppState;
use actix_web::http::header::*;
use actix_web::http::{HttpTryFrom, Method, StatusCode};
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{Body, FromRequest, HttpRequest, HttpResponse, Result};
use bigneon_db::models::*;
use bigneon_http::caching::*;
use itertools::Itertools;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

const CACHED_RESPONSE_HEADER: &'static str = "X-Cached-Response";

#[derive(PartialEq)]
pub enum OrganizationLoad {
    // /organizations/{id}/..
    Path,
}

#[derive(PartialEq)]
pub enum CacheUsersBy {
    // Logged in users and anonymous users receive cached results
    None,
    // Logged in users are not cached, anonymous users receive cached results
    AnonymousOnly,
    // Users are cached into groups according to the combination of roles on the users row
    // e.g. "Admin,Super", "Admin", "" is used for both logged in users with no roles and anon users
    // Organization access is not taken into account
    GlobalRoles,
    // Users are cached by their ID
    UserId,
    // Users are cached by their associated organization roles (cannot be used for event specific role endpoints)
    OrganizationScopePresence(OrganizationLoad, Scopes),
}

pub struct CacheResource {
    cache_users_by: CacheUsersBy,
}

impl CacheResource {
    pub fn new(cache_users_by: CacheUsersBy) -> CacheResource {
        CacheResource { cache_users_by }
    }
}

pub struct CacheConfiguration {
    cache_response: bool,
    served_cache: bool,
    error: bool,
    user_key: Option<String>,
    cache_data: BTreeMap<String, String>,
}

impl CacheConfiguration {
    pub fn new() -> CacheConfiguration {
        CacheConfiguration {
            cache_response: false,
            served_cache: false,
            error: false,
            user_key: None,
            cache_data: BTreeMap::new(),
        }
    }
}

impl Middleware<AppState> for CacheResource {
    fn start(&self, request: &HttpRequest<AppState>) -> Result<Started> {
        let mut cache_configuration = CacheConfiguration::new();
        if request.method() == Method::GET {
            let query_parameters = request.query().clone();
            for (key, value) in query_parameters.iter() {
                cache_configuration
                    .cache_data
                    .insert(key.to_string(), value.to_string());
            }
            let user_text = "x-user-role".to_string();
            cache_configuration
                .cache_data
                .insert("path".to_string(), request.path().to_string());
            cache_configuration
                .cache_data
                .insert("method".to_string(), request.method().to_string());
            let state = request.state().clone();
            let config = state.config.clone();

            if self.cache_users_by != CacheUsersBy::None {
                let user = match OptionalUser::from_request(request, &()) {
                    Ok(user) => user,
                    Err(error) => {
                        cache_configuration.error = true;
                        error!("CacheResource Middleware start: {:?}", error);
                        request.extensions_mut().insert(cache_configuration);
                        return Ok(Started::Done);
                    }
                };
                if let Some(user) = user.0 {
                    match &self.cache_users_by {
                        CacheUsersBy::None => (),
                        CacheUsersBy::AnonymousOnly => {
                            // Do not cache
                            return Ok(Started::Done);
                        }
                        CacheUsersBy::UserId => {
                            cache_configuration.user_key = Some(user.id().to_string());
                        }
                        CacheUsersBy::GlobalRoles => {
                            cache_configuration.user_key = Some(user.user.role.iter().map(|r| r.to_string()).join(","));
                        }
                        CacheUsersBy::OrganizationScopePresence(load_type, scope) => {
                            if let Some(connection) = request.extensions().get::<Connection>() {
                                let connection = connection.get();
                                match load_type {
                                    OrganizationLoad::Path => {
                                        // Assumes path element exists
                                        let organization_id: Uuid =
                                            request.match_info().get(&"id".to_string()).unwrap().parse().unwrap();
                                        let organization = match Organization::find(organization_id, connection) {
                                            Ok(organization) => organization,
                                            Err(error) => {
                                                cache_configuration.error = true;
                                                error!("CacheResource Middleware start: {:?}", error);
                                                request.extensions_mut().insert(cache_configuration);
                                                return Ok(Started::Done);
                                            }
                                        };

                                        let has_scope =
                                            match user.has_scope_for_organization(*scope, &organization, connection) {
                                                Ok(organization_scopes) => organization_scopes,
                                                Err(error) => {
                                                    cache_configuration.error = true;
                                                    error!("CacheResource Middleware start: {:?}", error);
                                                    request.extensions_mut().insert(cache_configuration);
                                                    return Ok(Started::Done);
                                                }
                                            };

                                        cache_configuration.user_key =
                                            Some(format!("{}-{}", scope, if has_scope { "t" } else { "f" }));
                                    }
                                }
                            } else {
                                cache_configuration.error = true;
                                error!("CacheResource Middleware start: unable to load connection");
                                request.extensions_mut().insert(cache_configuration);
                                return Ok(Started::Done);
                            }
                        }
                    }
                    if let Some(ref user_key) = cache_configuration.user_key {
                        cache_configuration.cache_data.insert(user_text, user_key.to_string());
                    }
                }
            }

            let cache_database = state.database.cache_database.clone();
            // if there is a error in the cache, the value does not exist
            let cached_value = cache_database
                .clone()
                .inner
                .clone()
                .and_then(|conn| caching::get_cached_value(conn, &config, &cache_configuration.cache_data));
            if let Some(response) = cached_value {
                // Insert self into extensions to let response know not to set the value
                cache_configuration.served_cache = true;
                request.extensions_mut().insert(cache_configuration);
                return Ok(Started::Response(response));
            }
        }

        cache_configuration.cache_response = true;
        request.extensions_mut().insert(cache_configuration);
        Ok(Started::Done)
    }

    fn response(&self, request: &HttpRequest<AppState>, mut response: HttpResponse) -> Result<Response> {
        let state = request.state().clone();
        let cache_database = state.database.cache_database.clone();
        let path = request.path().to_string();

        if request.method() == Method::GET {
            if response.status() == StatusCode::OK {
                let extensions = request.extensions();
                if let Some(cache_configuration) = extensions.get::<CacheConfiguration>() {
                    let config = state.config.clone();
                    if cache_configuration.cache_response {
                        cache_database.inner.clone().and_then(|conn| {
                            caching::set_cached_value(conn, &config, &response, &cache_configuration.cache_data).ok()
                        });
                    }

                    if cache_configuration.served_cache {
                        response.headers_mut().insert(
                            &HeaderName::try_from(CACHED_RESPONSE_HEADER).unwrap(),
                            HeaderValue::from_static("1"),
                        );
                    }

                    // If an error occurred fetching db data, do not send caching headers
                    if !cache_configuration.error {
                        // Cache headers for client
                        if let Ok(cache_control_header_value) = HeaderValue::from_str(&format!(
                            "{}, max-age={}",
                            if cache_configuration.user_key.is_none() {
                                "public"
                            } else {
                                "private"
                            },
                            config.client_cache_period
                        )) {
                            response
                                .headers_mut()
                                .insert(&CACHE_CONTROL, cache_control_header_value);
                        }

                        if let Ok(response_str) = application::unwrap_body_to_string(&response) {
                            if let Ok(payload) = serde_json::from_str::<Value>(&response_str) {
                                let etag_hash = etag_hash(&payload.to_string());
                                if let Ok(new_header_value) = HeaderValue::from_str(&etag_hash) {
                                    response.headers_mut().insert(&ETAG, new_header_value);
                                    if request.headers().contains_key(IF_NONE_MATCH) {
                                        let etag = ETag(EntityTag::weak(etag_hash.to_string()));
                                        if let Ok(header_value) = request.headers()[IF_NONE_MATCH].to_str() {
                                            let etag_header = ETag(EntityTag::weak(header_value.to_string()));
                                            if etag.weak_eq(&etag_header) {
                                                response.set_body(Body::Empty);
                                                *response.status_mut() = StatusCode::NOT_MODIFIED;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Method besides GET requested, PUT/POST/DELETE so clear any matching path cache
            match *request.method() {
                Method::PUT | Method::PATCH | Method::POST | Method::DELETE => {
                    if response.error().is_none() {
                        cache_database
                            .inner
                            .clone()
                            .and_then(|conn| caching::delete_by_key_fragment(conn, path).ok());
                    }
                }
                _ => (),
            }
        }

        Ok(Response::Done(response))
    }
}
