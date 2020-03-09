use crate::auth::user::User as AuthUser;
use crate::auth::TokenResponse;
use crate::db::Connection;
use crate::errors::*;
use crate::extractors::*;
use crate::helpers::application;
use crate::models::FacebookWebLoginToken;
use crate::server::AppState;
use actix_web::{HttpResponse, web::Data};
use bigneon_db::prelude::*;
use bigneon_db::validators::{append_validation_error, create_validation_error};
use facebook::error::FacebookError;
use facebook::nodes::Event as FBEvent;
use facebook::prelude::{CoverPhoto, FacebookClient, FBID};
use itertools::Itertools;
use log::Level::Debug;
use reqwest;
use serde_json;
use uuid::Uuid;
use validator::ValidationErrors;

const FACEBOOK_GRAPH_URL: &str = "https://graph.facebook.com";
#[derive(Deserialize)]
struct FacebookGraphResponse {
    id: String,
    first_name: String,
    last_name: String,
    email: Option<String>,
}

// TODO: Not covered by tests
pub fn web_login(
    (state, connection, auth_token, auth_user): (
        Data<AppState>,
        Connection,
        Json<FacebookWebLoginToken>,
        OptionalUser,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let url = format!("{}/me?fields=id,email,first_name,last_name", FACEBOOK_GRAPH_URL);
    let connection = connection.get();
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token.access_token))
        .send()?
        .text()?;

    jlog!(Debug, "Facebook Login Response", { "response": &response });

    let facebook_graph_response: FacebookGraphResponse = serde_json::from_str(&response)?;

    if auth_token.link_to_user_id {
        let auth_user = auth_user.into_inner();
        if auth_user.is_none() {
            return application::unauthorized_with_message("User must be logged in to link Facebook", auth_user, None);
        }

        let auth_user = auth_user.unwrap();
        auth_user.user.add_or_replace_external_login(
            Some(auth_user.user.id),
            facebook_graph_response.id.clone(),
            FACEBOOK_SITE.to_string(),
            auth_token.access_token.clone(),
            vec![],
            connection,
        )?;
        let response = TokenResponse::create_from_user(
            &state.config.token_secret,
            &state.config.token_issuer,
            &state.config.jwt_expiry_time,
            &auth_user.user,
        )?;
        return Ok(HttpResponse::Ok().json(response));
    }

    let existing_user = ExternalLogin::find_user(&facebook_graph_response.id, FACEBOOK_SITE, connection)?;
    let user = match existing_user {
        Some(u) => {
            let user = User::find(u.user_id, connection)?;
            if user.deleted_at.is_some() {
                return application::forbidden("This account has been deleted");
            }
            user
        }
        None => {
            if let Some(email) = facebook_graph_response.email.as_ref() {
                // Link account if email exists
                match User::find_by_email(&email, true, connection) {
                    Ok(user) => {
                        if user.deleted_at.is_some() {
                            return application::forbidden("This account has been deleted");
                        }
                        user.add_external_login(
                            None,
                            facebook_graph_response.id.clone(),
                            FACEBOOK_SITE.to_string(),
                            auth_token.access_token.clone(),
                            vec![],
                            connection,
                        )?;
                        user
                    }
                    Err(e) => {
                        match e.code {
                            // Not found
                            2000 => User::create_from_external_login(
                                facebook_graph_response.id.clone(),
                                facebook_graph_response.first_name.clone(),
                                facebook_graph_response.last_name.clone(),
                                facebook_graph_response.email.clone(),
                                FACEBOOK_SITE.to_string(),
                                auth_token.access_token.clone(),
                                vec![],
                                None,
                                connection,
                            )?,
                            _ => return Err(e.into()),
                        }
                    }
                }
            } else {
                User::create_from_external_login(
                    facebook_graph_response.id.clone(),
                    facebook_graph_response.first_name.clone(),
                    facebook_graph_response.last_name.clone(),
                    None,
                    FACEBOOK_SITE.to_string(),
                    auth_token.access_token.clone(),
                    vec![],
                    None,
                    connection,
                )?
            }
        }
    };
    let response = TokenResponse::create_from_user(
        &state.config.token_secret,
        &state.config.token_issuer,
        &state.config.jwt_expiry_time,
        &user,
    )?;
    Ok(HttpResponse::Ok().json(response))
}

pub fn request_manage_page_access(
    (_connection, state, user): (Connection, Data<AppState>, AuthUser),
) -> Result<HttpResponse, BigNeonError> {
    // TODO Sign/encrypt the user id passed through so that we can verify it has not been spoofed
    let redirect_url = FacebookClient::get_login_url(
        state.config.facebook_app_id.as_ref().ok_or_else(|| {
            ApplicationError::new_with_type(
                ApplicationErrorType::Unprocessable,
                "Facebook App ID has not been configured".to_string(),
            )
        })?,
        None,
        &user.id().to_string(),
        &["manage-pages"],
    );

    #[derive(Serialize)]
    struct R {
        redirect_url: String,
    };

    let r = R { redirect_url };

    Ok(HttpResponse::Ok().json(r))
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct AuthCallbackPathParameters {
    code: Option<String>,
    state: Option<String>,
    error_reason: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Returns a list of pages that a user has access to manage
pub fn pages((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let db_user = user.user;
    let fb_login = db_user.find_external_login(FACEBOOK_SITE, conn).optional()?;
    if fb_login.is_none() {
        return application::forbidden("User is not linked to Facebook");
    }
    let fb_login = fb_login.unwrap();

    let client = FacebookClient::from_user_access_token(fb_login.access_token.clone());
    let permissions = client.permissions.list(&fb_login.external_user_id)?;
    let list_pages_permission = permissions.data.iter().find(|p| p.permission == "pages_show_list");
    if list_pages_permission
        .as_ref()
        .map(|p| &p.status)
        .unwrap_or(&"declined".to_string())
        != "granted"
    {
        return application::forbidden("User must be granted access to view pages on Facebook");
    }

    let pages = client
        .me
        .accounts
        .list()?
        .into_iter()
        .map(|p| FacebookPage { id: p.id, name: p.name })
        .collect_vec();
    Ok(HttpResponse::Ok().json(pages))
}

#[derive(Serialize)]
pub struct FacebookPage {
    pub id: String,
    pub name: String,
}

pub fn scopes((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let db_user = user.user;
    let external_login = db_user.find_external_login(FACEBOOK_SITE, conn)?;
    Ok(HttpResponse::Ok().json(external_login.scopes))
}

pub fn disconnect((connection, user): (Connection, AuthUser)) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let db_user = user.user;
    let external_login = db_user.find_external_login(FACEBOOK_SITE, conn)?;
    external_login.delete(Some(db_user.id), conn)?;
    Ok(HttpResponse::Ok().finish())
}

pub fn create_event(
    (connection, user, data, state): (Connection, AuthUser, Json<CreateFacebookEvent>, Data<AppState>),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let config = &state.config;
    let event = Event::find(data.event_id, conn)?;
    if !event.is_published() {
        return Err(
            ApplicationError::unprocessable("Cannot create this event on Facebook until it is published").into(),
        );
    }
    let organization = event.organization(conn)?;
    user.requires_scope_for_organization_event(Scopes::EventWrite, &organization, &event, conn)?;

    if config.facebook_app_id.is_none() || config.facebook_app_secret.is_none() {
        return Err(ApplicationError::unprocessable("Facebook is not configured for use").into());
    }

    let client = FacebookClient::from_app_access_token(
        config.facebook_app_id.as_ref().unwrap(),
        config.facebook_app_secret.as_ref().unwrap(),
    )?;

    let mut validation_errors: Result<(), ValidationErrors> = Ok(());

    if event.promo_image_url.is_none() {
        validation_errors = append_validation_error(
            validation_errors,
            "promo_image_url",
            Err(create_validation_error(
                "promo_image_url",
                "Event must have a cover image to use as an image on Facebook",
            )),
        );
    }

    let _result = match validation_errors {
        Ok(_) => (),
        Err(e) => {
            let res: DatabaseError = e.into();
            return Err(res.into());
        }
    };
    let venue = event
        .venue(conn)?
        .ok_or_else(|| ApplicationError::unprocessable("Cannot publish this event on Facebook without a venue"))?;
    let mut fb_event = FBEvent::new(
        data.category.parse()?,
        data.title.clone(),
        data.description.clone(),
        venue.timezone.to_string(),
        event.promo_image_url.as_ref().map(|u| CoverPhoto::new(u.to_string())),
        event
            .get_all_localized_times(Some(&venue))
            .event_start
            .ok_or_else(|| {
                ApplicationError::unprocessable("Cannot publish this event in Facebook without a start time")
            })?
            .to_rfc3339(),
    );

    match data.location_type {
        EventLocationType::UsePageLocation => fb_event.place_id = Some(FBID(data.page_id.clone())),
        EventLocationType::CustomAddress => fb_event.address = data.custom_address.clone(),
    }

    fb_event.ticket_uri = Some(format!(
        "{}/tickets/{}?eventref=fb_oea",
        state.config.front_end_url,
        event.slug(conn)?
    ));

    fb_event.admins.push(data.page_id.clone());

    let fb_id = match client.official_events.create(fb_event) {
        Ok(i) => i,
        Err(err) => match err {
            FacebookError::FacebookError(e) => {
                if e.code == 100 && e.message.contains("place_id must be a valid place ID") {
                    return Err(ApplicationError::unprocessable("The page you specified does not have a location associated with it. Either specify an address explicitly, or add a street address to your Facebook page").into());
                }
                return Err(ApplicationError::unprocessable(&format!(
                    "Could not create event on Facebook. Facebook returned: ({}) {} [fbtrace_id:{}]",
                    e.code, &e.message, &e.fbtrace_id
                ))
                .into());
            }
            _ => return Err(err.into()),
        },
    };

    let mut attr = EventEditableAttributes::default();
    attr.facebook_event_id = Some(Some(fb_id.to_string()));
    event.update(Some(user.id()), attr, conn)?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize)]
pub struct CreateFacebookEvent {
    event_id: Uuid,
    page_id: String,
    title: String,
    category: String,
    description: String,
    location_type: EventLocationType,
    custom_address: Option<String>,
}

#[derive(Deserialize)]
pub enum EventLocationType {
    UsePageLocation,
    CustomAddress,
}
