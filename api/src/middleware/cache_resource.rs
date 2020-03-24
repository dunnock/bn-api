use crate::database::Connection;
use crate::extractors::*;
use crate::helpers::*;
use crate::server::GetAppState;
use crate::utils::logging::log_request;
use actix_service::Service;
use actix_web::error;
use actix_web::http::header::*;
use actix_web::http::{Method, StatusCode};
use actix_web::{dev, FromRequest, HttpRequest, HttpResponse};
use db::models::*;
use futures::future::{ok, Ready};
use http::caching::*;
use itertools::Itertools;
use log::Level;
use serde_json::Value;
use std::collections::BTreeMap;
use url::form_urlencoded;
use uuid::Uuid;

const CACHED_RESPONSE_HEADER: &'static str = "X-Cached-Response";
const CACHE_BYPASS_HEADER: &'static str = "Cache-Bypass";

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
    // Only public users (logged out or lacking organization_users) are cached
    PublicUsersOnly,
    // Users are cached by their associated organization roles (cannot be used for event specific role endpoints)
    OrganizationScopePresence(OrganizationLoad, Scopes),
}

enum Cache {
    Miss(CacheConfiguration),
    Hit(HttpResponse, CacheConfiguration),
    Skip,
    Timeout(CacheConfiguration),
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

    fn start_error(mut self, error: &str) -> Cache {
        self.error = true;
        error!("CacheResource Middleware start: {:?}", error);
        return Cache::Miss(self);
    }
}

impl CacheResource {
    pub fn new(cache_users_by: CacheUsersBy) -> Self {
        Self { cache_users_by }
    }

    // Identify caching action and data based on request
    // When resulting in Cache::Hit route handler will be skipped
    async fn start(&self, request: &HttpRequest) -> Cache {
        let mut cache_configuration = CacheConfiguration::new();
        if request.method() == Method::GET {
            if request
                .headers()
                .contains_key(CACHE_BYPASS_HEADER.parse::<HeaderName>().unwrap())
            {
                return Cache::Miss(cache_configuration);
            }
            for (key, value) in form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes()) {
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
                let user = match OptionalUser::from_request(request, &mut dev::Payload::None).into_inner() {
                    Ok(user) => user,
                    Err(error) => {
                        return cache_configuration.start_error(&format!("{:?}", error));
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
                        CacheUsersBy::PublicUsersOnly => {
                            if !user.is_public_user {
                                return Cache::Miss(cache_configuration);
                            }
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
                                                return cache_configuration.start_error(&format!("{:?}", error));
                                            }
                                        };

                                        let has_scope =
                                            match user.has_scope_for_organization(*scope, &organization, connection) {
                                                Ok(organization_scopes) => organization_scopes,
                                                Err(error) => {
                                                    return cache_configuration.start_error(&format!("{:?}", error));
                                                }
                                            };

                                        cache_configuration.user_key =
                                            Some(format!("{}-{}", scope, if has_scope { "t" } else { "f" }));
                                    }
                                }
                            } else {
                                return cache_configuration.start_error("unable to load connection");
                            }
                        }
                    }
                    if let Some(ref user_key) = cache_configuration.user_key {
                        cache_configuration.cache_data.insert(user_text, user_key.to_string());
                    }
                }
            }

            // if there is a error in the cache, the value does not exist
            let value =
                caching::get_cached_value(&state.database.cache_database, &config, &cache_configuration.cache_data)
                    .await;
            match value {
                Ok(Some(response)) => {
                    cache_configuration.served_cache = true;
                    return Cache::Hit(response, cache_configuration);
                }
                Err(e) if e.timeout => {
                    return Cache::Timeout(cache_configuration);
                }
                _ => {}
            }
        }

        cache_configuration.cache_response = true;
        Cache::Miss(cache_configuration)
    }

    // Updates cached data based on Cache result
    // This method will also issue unmodified when actual result did not change
    async fn update(
        cache_configuration: CacheConfiguration,
        mut response: dev::ServiceResponse,
    ) -> dev::ServiceResponse {
        let state = response.request().state();
        if state.database.cache_database.inner.is_none() {
            return response;
        }
        match *response.request().method() {
            Method::GET if response.status() == StatusCode::OK => {
                let config = state.config.clone();

                if cache_configuration.cache_response {
                    caching::set_cached_value(
                        &state.database.cache_database,
                        &config,
                        response.response(),
                        &cache_configuration.cache_data,
                    )
                    .await
                    .ok();
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

                    caching::delete_by_key_fragment(&state.database.cache_database, path)
                        .await
                        .ok();
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
        Box::pin(async move {
            let (http_req, payload) = request.into_parts();
            let cache = resource.start(&http_req).await;

            match cache {
                Cache::Hit(response, status) => {
                    log_request(
                        Level::Debug,
                        "api::cache_resource",
                        "Cache hit",
                        &http_req,
                        json!({"cache_result": "hit", "cache_user_key": status.user_key, "cache_response": status.cache_response, "cache_hit": true}),
                    );
                    let response = dev::ServiceResponse::new(http_req, response);
                    Ok(CacheResource::update(status, response).await)
                }
                Cache::Miss(status) => {
                    log_request(
                        Level::Debug,
                        "api::cache_resource",
                        "Cache miss",
                        &http_req,
                        json!({"cache_result": "miss", "cache_user_key": status.user_key, "cache_response": status.cache_response, "cache_hit": false}),
                    );
                    let request = dev::ServiceRequest::from_parts(http_req, payload)
                        .unwrap_or_else(|_| unreachable!("Failed to recompose request in CacheResourceService::call"));
                    let fut = service.borrow_mut().call(request);
                    let response = fut.await?;
                    Ok(CacheResource::update(status, response).await)
                }
                Cache::Skip => {
                    let request = dev::ServiceRequest::from_parts(http_req, payload)
                        .unwrap_or_else(|_| unreachable!("Failed to recompose request in CacheResourceService::call"));
                    let fut = service.borrow_mut().call(request);
                    fut.await
                }
                Cache::Timeout(status) => {
                    // When timing out from cache, we don't need to update it
                    log_request(
                        Level::Debug,
                        "api::cache_resource",
                        "Cache timeout",
                        &http_req,
                        json!({"cache_result": "timeout", "cache_user_key": status.user_key, "cache_response": status.cache_response, "cache_hit": false}),
                    );
                    let request = dev::ServiceRequest::from_parts(http_req, payload)
                        .unwrap_or_else(|_| unreachable!("Failed to recompose request in CacheResourceService::call"));
                    let fut = service.borrow_mut().call(request);
                    fut.await
                }
            }
        })
    }
}
