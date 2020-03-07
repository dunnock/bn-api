use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::http::HttpTryFrom;
use actix_web::middleware::{Middleware, Response};
use actix_web::Body::Binary;
use actix_web::{HttpRequest, HttpResponse, Result};
use regex::{Captures, Regex};
use serde_json;
use crate::server::AppState;
use std::str;

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

pub struct Metatags {
    trigger_header: String,
    trigger_value: String,
    front_end_url: String,
    app_name: String,
}

impl Metatags {
    pub fn new(trigger_header: String, trigger_value: String, front_end_url: String, app_name: String) -> Metatags {
        Metatags {
            trigger_header,
            trigger_value,
            front_end_url,
            app_name,
        }
    }
}

impl Middleware<AppState> for Metatags {
    fn response(&self, req: &HttpRequest<AppState>, mut resp: HttpResponse) -> Result<Response> {
        if resp.status() != 200 {
            return Ok(Response::Done(resp));
        }
        let resp = match req.headers().get(&self.trigger_header) {
            Some(header) => {
                if header.to_str().unwrap_or("").to_string() == self.trigger_value {
                    let mut values: HashMap<&str, String> = HashMap::new();
                    let path = req.uri().path();

                    let mut data_to_use: Option<Value> = None;
                    //Check we have hit an event view endpoint
                    //TODO move this into a customizable format
                    let event_re = Regex::new(r"/events/[A-Za-z0-9\-]{36}$").unwrap();
                    if event_re.is_match(path) {
                        data_to_use = match resp.body() {
                            Binary(binary) => {
                                let json =
                                    serde_json::from_str(str::from_utf8(binary.as_ref()).unwrap()).unwrap_or(None);
                                if json.is_some() {
                                    Some(json.unwrap())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        }
                    }

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
                    resp.headers_mut().insert(
                        HeaderName::try_from(CONTENT_TYPE).unwrap(),
                        HeaderValue::from_static(TEXT_HTML),
                    );

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

                    resp.set_body(result);
                }
                resp
            }
            None => resp,
        };
        return Ok(Response::Done(resp));
    }
}
