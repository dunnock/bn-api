use actix_web::{test, FromRequest, HttpRequest, web::{Path, Query, Data}};
use bigneon_api::config::Config;
use bigneon_api::db::Database;
use bigneon_api::server::AppState;
use bigneon_api::utils::spotify;
use bigneon_db::models::Environment;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct TestRequest {
    pub request: HttpRequest,
    pub config: Config,
}

impl TestRequest {
    pub fn create() -> TestRequest {
        TestRequest::create_with_uri("/")
    }

    pub fn create_with_uri(path: &str) -> TestRequest {
        TestRequest::create_with_uri_custom_params(path, vec!["id"])
    }

    pub fn create_with_uri_custom_params(path: &str, params: Vec<&'static str>) -> TestRequest {
        let mut config = Config::new(Environment::Test);
        config.token_secret = "test_secret".into();
        config.token_issuer = "bn-api-test".into();
        config.api_keys_encryption_key = "test_encryption_key".to_string();
        config.google_recaptcha_secret_key = None;
        if config.spotify_auth_token.is_some() {
            spotify::SINGLETON.set_auth_token(&config.spotify_auth_token.clone().unwrap());
        }

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let test_request = test::TestRequest::with_state(
            AppState::new(
                config.clone(),
                Database::from_config(&config),
                Database::readonly_from_config(&config),
                clients,
            )
            .expect("Failed to generate app state for testing"),
        );

        // TODO: actix-web test requests do not allow router customization except
        // within crate. Forcing an ID here so the extractor can still build the
        // parameters wrapped in the Path struct. Should refactor when they settle
        // on a final test request design as the current does not support extractors.

        let mut request = test_request.uri(path);

        for param in params {
            request = request.param(param, "0f85443e-9e70-45ba-bf28-0f59c183856f");
        }

        TestRequest {
            request: request.finish(),
            config,
        }
    }

    pub fn extract_state(&self) -> Data<AppState> {
        Data::<AppState>::extract(&self.request)
    }
}

pub struct RequestBuilder {
    request: TestRequest,
}

impl RequestBuilder {
    pub fn new(uri: &str) -> Self {
        let request = TestRequest::create_with_uri(&uri);
        RequestBuilder { request }
    }

    pub fn state(&self) -> State<AppState> {
        self.request.extract_state()
    }

    pub fn path<P>(&self) -> Path<P>
    where
        P: DeserializeOwned,
    {
        Path::<P>::extract(&self.request.request).unwrap()
    }

    pub fn query<Q>(&self) -> Query<Q>
    where
        Q: DeserializeOwned,
    {
        Query::<Q>::extract(&self.request.request).unwrap()
    }
}
