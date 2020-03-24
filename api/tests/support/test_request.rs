use actix_web::{
    test,
    web::{Data, Path, Query},
    FromRequest, HttpRequest,
};
use api::auth::default_token_issuer::DefaultTokenIssuer;
use api::config::Config;
use api::database::Database;
use api::server::AppState;
use api::utils::spotify;
use db::models::Environment;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    static ref DB: Arc<Mutex<Option<Database>>> = Arc::new(Mutex::new(None));
    static ref RO_DB: Arc<Mutex<Option<Database>>> = Arc::new(Mutex::new(None));
}

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
        config.token_issuer = Box::new(DefaultTokenIssuer::new("test_secret".into(), "bn-api-test".into()));
        config.api_keys_encryption_key = "test_encryption_key".to_string();
        config.google_recaptcha_secret_key = None;
        if config.spotify_auth_token.is_some() {
            spotify::SINGLETON.set_auth_token(&config.spotify_auth_token.clone().unwrap());
        }

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let dbs = get_dbs(&config);
        let test_request = test::TestRequest::get().data(
            AppState::new(config.clone(), dbs.0, dbs.1, clients).expect("Failed to generate app state for testing"),
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
            request: request.to_http_request(),
            config,
        }
    }

    pub async fn extract_state(&self) -> Data<AppState> {
        Data::extract(&self.request).await.unwrap()
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

    pub async fn state(&self) -> Data<AppState> {
        self.request.extract_state().await
    }

    pub async fn path<P>(&self) -> Path<P>
    where
        P: DeserializeOwned,
    {
        Path::<P>::extract(&self.request.request).await.unwrap()
    }

    pub async fn query<Q>(&self) -> Query<Q>
    where
        Q: DeserializeOwned,
    {
        Query::<Q>::extract(&self.request.request).await.unwrap()
    }
}

fn get_dbs(config: &Config) -> (Database, Database) {
    let get_db = |db: Arc<Mutex<Option<Database>>>| async {
        let mut db_guard = db.lock().unwrap();
        if let Some(ref db) = *db_guard {
            return db.clone();
        };
        *db_guard = Some(Database::from_config(config).await);
        db_guard.as_ref().unwrap().clone()
    };
    let rt = tokio::runtime::Runtime().new();
    rt.block_on(async {
        (get_db(DB.clone()).await, get_db(RO_DB.clone()).await)
    })
    
}
