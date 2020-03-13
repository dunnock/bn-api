use crate::config::Config;
use crate::helpers::application::extract_response_bytes;
use actix_service::Service;
use actix_web::http::header::HeaderValue;
use actix_web::{dev, error};
use regex::{Captures, Regex};
use serde_json;
use std::str;

use futures::future::{ok, Ready};
use serde_json::Value;
use std::collections::HashMap;

const CONTENT_TYPE: &'static str = "Content-Type";
const TEXT_HTML: &'static str = "text/html";

const HTML_RESPONSE: &'static str = r#"
<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">

    <title>%title%</title>
    <meta name="description" content="%description%">
    <meta name="author" content="%creator%">

    <meta property="og:type" content="website"/>
    <meta property="og:title" content="%title%"/>
    <meta property="og:url" content="%url%"/>
    <meta property="og:image" content="%promo_image_url%"/>
    <meta property="og:site_name" content="%site_name%"/>
    <meta property="og:description" content="%description%"/>

    <meta name="twitter:site" content="%url%"/>
    <meta name="twitter:creator" content="%creator%"/>
    <meta name="twitter:title" content="%title%"/>
    <meta name="twitter:image" content="%promo_image_url%"/>
    <meta name="description" content="%description%"/>
</head>

<body>
</body>
</html>"#;

#[derive(Clone)]
pub struct Metatags {
    trigger_header: String,
    trigger_value: String,
    front_end_url: String,
    app_name: String,
}

impl Metatags {
    pub fn new(config: &Config) -> Metatags {
        Metatags {
            trigger_header: config.ssr_trigger_header.clone(),
            trigger_value: config.ssr_trigger_value.clone(),
            front_end_url: config.front_end_url.clone(),
            app_name: config.app_name.clone(),
        }
    }

    async fn process<B: dev::MessageBody>(&self, mut response: dev::ServiceResponse<B>) -> dev::ServiceResponse<B> {
        if !response.status().is_success() {
            return response;
        }
        let headers = response.request().headers();
        if let Some(header) = headers.get(&self.trigger_header) {
            if header.to_str().unwrap_or("").to_string() == self.trigger_value {
                let mut values: HashMap<&str, String> = HashMap::new();
                let path = response.request().uri().path().to_owned();

                //Check we have hit an event view endpoint
                //TODO move this into a customizable format
                let event_re = Regex::new(r"/events/[A-Za-z0-9\-]{36}$").unwrap();
                let data_to_use: Option<Value> = if event_re.is_match(path.as_str()) {
                    extract_response_bytes(&mut response.take_body())
                        .await
                        .map(|body| serde_json::from_slice(&body).ok())
                        .ok()
                        .flatten()
                } else {
                    None
                };

                values.insert("title", format!("{}", self.app_name));
                values.insert("site_name", format!("{}", self.app_name));
                values.insert("url", format!("{}{}", self.front_end_url, path));
                values.insert("creator", format!("{}", self.app_name));
                //TODO add the slogan to the .env
                values.insert("description", format!("{}", "The Future Of Ticketing"));
                //TODO Add the logo address to the .env
                values.insert(
                    "promo_image_url",
                    format!("{}{}", self.front_end_url, "/images/bn-logo-text-web.svg"),
                );
                if let Some(data) = data_to_use {
                    let name = data["name"].as_str().unwrap_or("");
                    let description = data["additional_info"].as_str().unwrap_or("");
                    let promo_image_url = data["promo_image_url"].as_str().unwrap_or("");
                    let creator = data["venue"]["name"].as_str().unwrap_or("");

                    values.entry("title").and_modify(|e| {
                        *e = format!("{} - {}", self.app_name, name);
                    });
                    values.entry("description").and_modify(|e| {
                        *e = format!("{}", description);
                    });
                    values.entry("creator").and_modify(|e| {
                        *e = format!("{}", creator);
                    });
                    values.entry("promo_image_url").and_modify(|e| {
                        *e = format!("{}", promo_image_url);
                    });
                }
                response
                    .headers_mut()
                    .insert(CONTENT_TYPE.parse().unwrap(), HeaderValue::from_static(TEXT_HTML));

                let keys = vec!["creator", "description", "promo_image_url", "site_name", "title", "url"];

                let mut result = HTML_RESPONSE.to_string();

                for key in keys.into_iter() {
                    let regex_expression = format!("%{}%", key);
                    let re = Regex::new(regex_expression.as_str()).unwrap();
                    let value = values.get(key).map(|v| v.to_string()).unwrap_or("".to_string());
                    result = re
                        .replace_all(result.as_str(), |_caps: &Captures| format!("{}", value))
                        .to_string();
                }

                return response.map_body(|_, _| dev::ResponseBody::Other(dev::Body::from(result)));
            }
        };
        response
    }
}

impl<S, B> dev::Transform<S> for Metatags
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type InitError = ();
    type Transform = MetatagsService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(MetatagsService::new(service, self.clone()))
    }
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct MetatagsService<S> {
    service: S,
    metatags: Metatags,
}

impl<S> MetatagsService<S> {
    fn new(service: S, metatags: Metatags) -> Self {
        Self { service, metatags }
    }
}

impl<S, B> Service for MetatagsService<S>
where
    S: Service<Request = dev::ServiceRequest, Response = dev::ServiceResponse<B>, Error = error::Error> + 'static,
    B: dev::MessageBody,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(error::Error::from)
    }

    fn call(&mut self, request: Self::Request) -> Self::Future {
        let fut = self.service.call(request);
        let metatags = self.metatags.clone();
        Box::pin(async move {
            let response = fut.await?;
            Ok(metatags.process(response).await)
        })
    }
}
