use crate::db::Connection;
use crate::extractors::*;
use crate::helpers::*;
use crate::server::GetAppState;
use actix_service::Service;
use actix_web::error;
use actix_web::http::header::*;
use actix_web::http::{Method, StatusCode};
use actix_web::{dev, FromRequest, HttpRequest, HttpResponse};
use bigneon_db::models::*;
use bigneon_http::caching::*;
use futures::future::{ok, Ready};
use itertools::Itertools;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;
use log::Level;
use logging::jlog;

const CACHED_RESPONSE_HEADER: &'static str = "X-Cached-Response";

#[derive(PartialEq, Clone)]
pub enum OrganizationLoad {
    // /organizations/{id}/..
    Path,
}

#[derive(PartialEq, Clone)]
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

enum Cache {
    Miss(CacheConfiguration),
    Hit(HttpResponse, CacheConfiguration),
    Skip,
}

#[derive(Clone)]
pub struct CacheResource {
    pub cache_users_by: CacheUsersBy,
}

struct CacheConfiguration {
    cache_response: bool,
    served_cache: bool,
    error: bool,
    user_key: Option<String>,
    cache_data: BTreeMap<String, String>,
}

impl CacheConfiguration {
    fn new() -> CacheConfiguration {
        CacheConfiguration {
            cache_response: false,
            served_cache: false,
            error: false,
            user_key: None,
            cache_data: BTreeMap::new(),
        }
    }
}

impl CacheResource {
    pub fn new(cache_users_by: CacheUsersBy) -> Self {
        Self { cache_users_by }
    }

    // Identify caching action and data based on request
    // When resulting in Cache::Hit route handler will be skipped
    fn start(&self, request: &HttpRequest) -> Cache {
        let mut cache_configuration = CacheConfiguration::new();
        if request.method() == Method::GET {
            if let Ok(url) = url::Url::parse(&request.uri().to_string()) {
                for (key, value) in url.query_pairs() {
                    cache_configuration
                        .cache_data
                        .insert(key.to_string(), value.to_string());
                }
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
                let user = match OptionalUser::from_request(request, &mut dev::Payload::None).into_inner() {
                    Ok(user) => user,
                    Err(error) => {
                        cache_configuration.error = true;
                        error!("CacheResource Middleware start: {:?}", error);
                        return Cache::Miss(cache_configuration);
                    }
                };
                if let Some(user) = user.0 {
                    match &self.cache_users_by {
                        CacheUsersBy::None => (),
                        CacheUsersBy::AnonymousOnly => {
                            // Do not cache
                            return Cache::Skip;
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
                                                return Cache::Miss(cache_configuration);
                                            }
                                        };

                                        let has_scope =
                                            match user.has_scope_for_organization(*scope, &organization, connection) {
                                                Ok(organization_scopes) => organization_scopes,
                                                Err(error) => {
                                                    cache_configuration.error = true;
                                                    error!("CacheResource Middleware start: {:?}", error);
                                                    return Cache::Miss(cache_configuration);
                                                }
                                            };

                                        cache_configuration.user_key =
                                            Some(format!("{}-{}", scope, if has_scope { "t" } else { "f" }));
                                    }
                                }
                            } else {
                                cache_configuration.error = true;
                                error!("CacheResource Middleware start: unable to load connection");
                                return Cache::Miss(cache_configuration);
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
                return Cache::Hit(response, cache_configuration);
            }
        }

        cache_configuration.cache_response = true;
        Cache::Miss(cache_configuration)
    }

    // Updates cached data based on Cache result
    // This method will also issue unmodified when actual result did not change
    fn update(cache_configuration: CacheConfiguration, mut response: dev::ServiceResponse) -> dev::ServiceResponse {
        match *response.request().method() {
            Method::GET if response.status() == StatusCode::OK => {
                let state = response.request().state();
                let cache_database = state.database.cache_database.clone();
                let config = state.config.clone();

                if cache_configuration.cache_response {
                    cache_database.inner.clone().and_then(|conn| {
                        caching::set_cached_value(conn, &config, response.response(), &cache_configuration.cache_data)
                            .ok()
                    });
                }

                if cache_configuration.served_cache {
                    response
                        .headers_mut()
                        .insert(CACHED_RESPONSE_HEADER.parse().unwrap(), HeaderValue::from_static("1"));
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
                        response.headers_mut().insert(CACHE_CONTROL, cache_control_header_value);
                    }

                    if let Ok(response_str) = application::unwrap_body_to_string(response.response()) {
                        if let Ok(payload) = serde_json::from_str::<Value>(&response_str) {
                            let etag_hash = etag_hash(&payload.to_string());
                            if let Ok(new_header_value) = HeaderValue::from_str(&etag_hash) {
                                response.headers_mut().insert(ETAG, new_header_value);
                                let headers = response.request().headers();
                                if headers.contains_key(IF_NONE_MATCH) {
                                    let etag = ETag(EntityTag::weak(etag_hash.to_string()));
                                    let if_none_match = headers.get(IF_NONE_MATCH).map(|h| h.to_str().ok());
                                    if let Some(Some(header_value)) = if_none_match {
                                        let etag_header = ETag(EntityTag::weak(header_value.to_string()));
                                        if etag.weak_eq(&etag_header) {
                                            return response.into_response(HttpResponse::NotModified().finish());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Method::PUT | Method::PATCH | Method::POST | Method::DELETE => {
                if response.response().error().is_none() {
                    let path = response.request().path().to_owned();
                    let state = response.request().state();
                    let cache_database = state.database.cache_database.clone();

                    cache_database
                        .inner
                        .clone()
                        .and_then(|conn| caching::delete_by_key_fragment(conn, path).ok());
                }
            }
            _ => (),
        };

        response
    }
}

impl<S> dev::Transform<S> for CacheResource
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse, Error = error::Error> + 'static,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type InitError = ();
    type Transform = CacheResourceService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let cache_users_by = self.cache_users_by.clone();
        let resource = CacheResource { cache_users_by };
        ok(CacheResourceService::new(service, resource))
    }
}

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub struct CacheResourceService<S> {
    service: Rc<RefCell<S>>,
    resource: CacheResource,
}

impl<S> CacheResourceService<S> {
    fn new(service: S, resource: CacheResource) -> Self {
        Self {
            service: Rc::new(RefCell::new(service)),
            resource,
        }
    }
}

impl<S> Service for CacheResourceService<S>
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse, Error = error::Error> + 'static,
{
    type Request = S::Request;
    type Response = dev::ServiceResponse;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.borrow_mut().poll_ready(cx).map_err(error::Error::from)
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let service = self.service.clone();
        let resource = self.resource.clone();
        let (http_req, payload) = request.into_parts();
        let cache = resource.start(&http_req);

        match cache {
            Cache::Hit(response, status) => {
                jlog!(Level::Debug, "bigneon_api::cache_resource", "Cache hit");
                let response = dev::ServiceResponse::new(http_req, response);
                Box::pin(async move {
                    Ok(CacheResource::update(status, response))
                })
            },
            Cache::Miss(status) => {
                jlog!(Level::Debug, "bigneon_api::cache_resource", "Cache miss");
                let request = dev::ServiceRequest::from_parts(http_req, payload)
                    .unwrap_or_else(|_| unreachable!("Failed to recompose request in CacheResourceService::call"));
                let fut = service.borrow_mut().call(request);
                Box::pin(async move {
                    let response = fut.await?;
                    Ok(CacheResource::update(status, response))
                })
            }
            Cache::Skip => {
                let request = dev::ServiceRequest::from_parts(http_req, payload)
                    .unwrap_or_else(|_| unreachable!("Failed to recompose request in CacheResourceService::call"));
                let fut = service.borrow_mut().call(request);
                Box::pin(fut)
            }
        }
    }
}
