use actix_web::{http::header, HttpRequest, HttpResponse, Query, State};
use bigneon_db::models::analytics::PageView;
use bigneon_db::prelude::*;
use chrono::prelude::*;
use crate::db::Connection;
use crate::errors::BigNeonError;
use itertools::Itertools;
use crate::server::AppState;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct PageViewTrackingData {
    event_id: Uuid,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    source: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    medium: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    term: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    platform: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    campaign: Option<String>,
    url: String,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    client_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_unless_blank")]
    referrer: Option<String>,
}

pub fn track(
    (state, query, request, connection): (
        State<AppState>,
        Query<PageViewTrackingData>,
        HttpRequest<AppState>,
        Connection,
    ),
) -> Result<HttpResponse, BigNeonError> {
    let conn = connection.get();
    let ip_address = request.connection_info().remote().map(|i| i.to_string());
    let user_agent = if let Some(ua) = request.headers().get(header::USER_AGENT) {
        let s = ua.to_str()?;
        Some(s.to_string())
    } else {
        None
    };

    let platform = Platforms::from_user_agent(user_agent.as_ref().map(|ua| ua.as_str()).unwrap())
        .map(|p| p.to_string())
        .ok();

    let url = Url::parse(&state.config.front_end_url)?;
    let url = url.join(query.url.to_string().as_str())?;
    let params = url.query_pairs().into_owned().collect_vec();

    let utm_source = extract_param(&params, "utm_source");
    let utm_medium = extract_param(&params, "utm_medium");
    let utm_content = extract_param(&params, "utm_content");
    let utm_term = extract_param(&params, "utm_term");
    let utm_campaign = extract_param(&params, "utm_campaign");
    let utm_code = extract_param(&params, "code");
    let referrer_host = match query.referrer {
        Some(ref r) => Url::parse(r)?.host_str().map(|h| h.to_string()),
        None => None,
    };
    let is_facebook = extract_param(&params, "fbclid").map(|_| "facebook.com".to_string());

    PageView::create(
        Utc::now().naive_utc(),
        query.event_id,
        query
            .source
            .clone()
            .or(utm_source)
            .or(referrer_host)
            .or(is_facebook.clone())
            .unwrap_or("direct".to_string()),
        query
            .medium
            .clone()
            .or(utm_medium)
            .or(query.referrer.as_ref().map(|_| "referral".to_string()))
            .or(is_facebook.map(|_| "referral".to_string()))
            .unwrap_or("".to_string()),
        query.term.clone().or(utm_term).unwrap_or("".to_string()),
        query.content.clone().or(utm_content).unwrap_or("".to_string()),
        query.platform.clone().or(platform).unwrap_or("".to_string()),
        query.campaign.clone().or(utm_campaign).unwrap_or("".to_string()),
        query.url.clone(),
        query.client_id.clone().unwrap_or("".to_string()),
        query.code.clone().or(utm_code).unwrap_or("".to_string()),
        ip_address.unwrap_or("".to_string()),
        user_agent.unwrap_or("".to_string()),
        query.referrer.clone().unwrap_or("".to_string()),
    )
    .commit(conn)?;

    Ok(HttpResponse::Ok().finish())
}

fn extract_param<'a>(params: &[(String, String)], name: &str) -> Option<String> {
    params.iter().find(|i| &i.0 == name).map(|i| i.1.clone())
}
