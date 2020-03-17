use actix_web::{http::StatusCode, HttpResponse, Path, Query};
use auth::user::User;
use bigneon_db::models::*;
use db::Connection;
use errors::*;
use extractors::*;
use models::PathParameters;
use models::WebPayload;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct EngagementData {
    pub action: Option<AnnouncementEngagementAction>,
}

pub fn index(
    (connection, query_parameters, user): (Connection, Query<PagingParameters>, User),
) -> Result<WebPayload<Announcement>, BigNeonError> {
    user.requires_scope(Scopes::AnnouncementRead)?;

    Ok(WebPayload::new(
        StatusCode::OK,
        Announcement::all(
            query_parameters.page() as i64,
            query_parameters.limit() as i64,
            connection.get(),
        )?,
    ))
}

pub fn show(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::AnnouncementRead)?;

    let announcement = Announcement::find(parameters.id, false, connection.get())?;
    Ok(HttpResponse::Ok().json(&announcement))
}

pub fn create(
    (connection, new_announcement, user): (Connection, Json<NewAnnouncement>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::AnnouncementWrite)?;
    let connection = connection.get();
    let announcement = new_announcement.into_inner().commit(Some(user.id()), connection)?;
    Ok(HttpResponse::Created().json(&announcement))
}

pub fn destroy(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::AnnouncementDelete)?;
    let connection = connection.get();
    let announcement = Announcement::find(parameters.id, false, connection)?;

    announcement.delete(Some(user.id()), connection)?;
    Ok(HttpResponse::Ok().json({}))
}

pub fn show_from_organization(
    (connection, parameters, user): (Connection, Path<PathParameters>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let organization = Organization::find(parameters.id, connection)?;
    user.requires_scope_for_organization(Scopes::AnnouncementEngagementWrite, &organization, connection)?;

    let announcements = Announcement::find_active_for_organization_user(organization.id, user.id(), connection)?;
    Ok(HttpResponse::Ok().json(&announcements))
}

pub fn engage(
    (connection, parameters, engagement_data, user): (Connection, Path<PathParameters>, Json<EngagementData>, User),
) -> Result<HttpResponse, BigNeonError> {
    let connection = connection.get();
    let announcement = Announcement::find(parameters.id, false, connection)?;
    if let Some(organization_id) = announcement.organization_id {
        let organization = Organization::find(organization_id, connection)?;
        user.requires_scope_for_organization(Scopes::AnnouncementEngagementWrite, &organization, connection)?;
    }
    AnnouncementEngagement::create(
        user.id(),
        announcement.id,
        engagement_data.action.unwrap_or(AnnouncementEngagementAction::Dismiss),
    )
    .commit(connection)?;

    Ok(HttpResponse::Ok().json(json!({})))
}

pub fn update(
    (connection, parameters, announcement_parameters, user): (
        Connection,
        Path<PathParameters>,
        Json<AnnouncementEditableAttributes>,
        User,
    ),
) -> Result<HttpResponse, BigNeonError> {
    user.requires_scope(Scopes::AnnouncementWrite)?;
    let connection = connection.get();
    let announcement = Announcement::find(parameters.id, false, connection)?;
    let updated_announcement = announcement.update(announcement_parameters.into_inner(), connection)?;
    Ok(HttpResponse::Ok().json(updated_announcement))
}
