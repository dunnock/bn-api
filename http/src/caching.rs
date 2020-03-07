pub use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag};
use actix_web::{dev::HttpResponseBuilder, http::header, HttpRequest, HttpResponse};
use serde::Serialize;

pub struct CacheHeaders(pub header::CacheControl, pub Option<ETag>);

pub trait ToETag {
    fn to_etag(&self) -> header::ETag;
}

impl CacheHeaders {
    pub fn cache_control(&self) -> &header::CacheControl {
        &self.0
    }

    pub fn etag(&self) -> &Option<ETag> {
        &self.1
    }

    pub fn into_response_json<P: Serialize>(self, req: &HttpRequest, payload: &P) -> HttpResponse {
        let (is_stale, mut builder) = self.into_response_builder(req);

        if is_stale {
            builder.json(payload)
        } else {
            builder.finish()
        }
    }

    /// Returns a HTTP 200 for stale requests, otherwise an HTTP 304 (NotModified) for
    /// requests where If-None-Match headers weakly match the ETag
    pub fn into_response_builder(self, req: &HttpRequest) -> (bool, HttpResponseBuilder) {
        let is_stale = self.is_stale(req);
        let mut builder = if is_stale {
            HttpResponse::Ok()
        } else {
            HttpResponse::NotModified()
        };

        self.set_headers(&mut builder);
        (is_stale, builder)
    }

    pub fn set_headers(&self, builder: &mut HttpResponseBuilder) {
        builder.set(self.cache_control().clone());
        if let Some(etag) = self.etag() {
            builder.set(etag.clone());
        }
    }

    pub fn is_stale(&self, req: &HttpRequest) -> bool {
        let if_none_match: Result<header::IfNoneMatch, _> = header::Header::parse(req);
        let if_none_match = if_none_match.ok();

        let etag = self.etag();
        let etag = match etag {
            Some(e) => e,
            None => return true,
        };

        match if_none_match {
            Some(h) => match h {
                header::IfNoneMatch::Items(entities) => !entities.iter().any(|e| etag.weak_eq(e)),
                header::IfNoneMatch::Any => true,
            },
            None => true,
        }
    }
}

pub fn etag_hash(s: &str) -> String {
    sha1::digest(s)
}

pub mod sha1 {
    use ring::digest;

    pub fn digest(s: &str) -> String {
        let sha = digest::digest(&digest::SHA1, s.as_bytes());
        sha.as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<String>>()
            .join("")
    }

    #[test]
    fn sha1_digest() {
        let sha = digest("testme");
        assert_eq!(sha, "3abef1a14ccecd20d6ce892cbe042ae6d74946c8");
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::test;
    use std::collections::hash_map::HashMap;

    type State = HashMap<String, String>;

    #[test]
    fn is_stale_no_etag() {
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(ETag(EntityTag::weak("abcd123".to_string()))),
        );

        let test_request = test::TestRequest::get().data(State::new()).to_http_request();

        assert!(subject.is_stale(&test_request));
    }

    #[test]
    fn is_stale_etag_match_any() {
        let etag = ETag(EntityTag::weak("abcd123".to_string()));
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(etag),
        );

        let hdr = header::IfNoneMatch::Any;
        let test_request = test::TestRequest::with_hdr(hdr).to_http_request();

        assert!(subject.is_stale(&test_request));
    }

    #[test]
    fn is_stale_etag_mismatch() {
        let etag = ETag(EntityTag::weak("abcd123".to_string()));
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(etag),
        );

        let hdr = header::IfNoneMatch::Items(vec![EntityTag::weak("abcd124".to_string())]);
        let test_request = test::TestRequest::with_hdr(hdr).to_http_request();

        assert!(subject.is_stale(&test_request));
    }

    #[test]
    fn is_stale_etag_matches() {
        let etag = ETag(EntityTag::weak("abcd123".to_string()));
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(etag),
        );

        let hdr = header::IfNoneMatch::Items(vec![EntityTag::weak("abcd123".to_string())]);
        let test_request = test::TestRequest::with_hdr(hdr).to_http_request();

        assert!(!subject.is_stale(&test_request));
    }

    #[test]
    fn into_response_found_304() {
        let etag = ETag(EntityTag::weak("abcd123".to_string()));
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(etag),
        );

        let hdr = header::IfNoneMatch::Items(vec![EntityTag::weak("abcd123".to_string())]);
        let test_request = test::TestRequest::with_hdr(hdr).to_http_request();

        let (is_stale, mut builder) = subject.into_response_builder(&test_request);
        let resp = builder.finish();
        assert!(!is_stale);

        assert_eq!(resp.status().as_u16(), 304);

        let headers = resp.headers().clone();

        let cache_control_header = headers.get("Cache-Control");
        assert!(cache_control_header.is_some());

        let headers = resp.headers().clone();
        let etag_header = headers.get("ETag");
        assert!(etag_header.is_some());
    }

    #[test]
    fn into_response_ok() {
        let etag = ETag(EntityTag::weak("abcd123".to_string()));
        let subject = CacheHeaders(
            CacheControl(vec![CacheDirective::MaxAge(60u32), CacheDirective::Public]),
            Some(etag),
        );

        let hdr = header::IfNoneMatch::Items(vec![EntityTag::weak("abcd124".to_string())]);
        let test_request = test::TestRequest::with_hdr(hdr).to_http_request();

        let (is_stale, mut builder) = subject.into_response_builder(&test_request);
        let resp = builder.finish();
        assert!(is_stale);

        assert_eq!(resp.status().as_u16(), 200u16);

        let headers = resp.headers().clone();

        let cache_control_header = headers.get("Cache-Control");
        assert!(cache_control_header.is_some());

        let headers = resp.headers().clone();
        let etag_header = headers.get("ETag");
        assert!(etag_header.is_some());
    }
}
