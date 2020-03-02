use auth::default_token_issuer::DefaultTokenIssuer;
use bigneon_db::models::{EmailProvider, Environment};
use bigneon_db::utils::errors::EnumParseError;
use chrono::Duration;
use dotenv::dotenv;
use errors::{ApplicationError, BigNeonError};
use itertools::Itertools;
use std::env;
use std::fmt;
use std::str;
use std::str::FromStr;
use tari_client::{HttpTariClient, TariClient, TariTestClient};

#[derive(Clone)]
pub struct Config {
    pub actix: Actix,
    pub allowed_origins: String,
    pub front_end_url: String,
    pub api_host: String,
    pub api_port: String,
    pub app_name: String,
    pub cube_js: CubeJs,
    pub database_url: String,
    pub redis_connection_string: Option<String>,
    pub redis_connection_timeout: u64,
    pub redis_read_timeout: u64,
    pub redis_write_timeout: u64,
    pub readonly_database_url: String,
    pub redis_cache_period: Option<usize>,
    pub domain: String,
    pub email_templates: EmailTemplates,
    pub environment: Environment,
    pub facebook_app_id: Option<String>,
    pub facebook_app_secret: Option<String>,
    pub globee_api_key: String,
    pub globee_base_url: String,
    pub validate_ipns: bool,
    pub api_base_url: String,
    pub google_recaptcha_secret_key: Option<String>,
    pub http_keep_alive: usize,
    pub block_external_comms: bool,
    pub primary_currency: String,
    pub stripe_secret_key: String,
    pub token_issuer: Box<DefaultTokenIssuer>,
    pub tari_client: Box<dyn TariClient + Send + Sync>,
    pub communication_default_source_email: String,
    pub communication_default_source_phone: String,
    pub sendgrid_api_key: String,
    pub sendgrid_template_bn_refund: String,
    pub sendgrid_template_bn_user_registered: String,
    pub sendgrid_template_bn_purchase_completed: String,
    pub sendgrid_template_bn_cancel_transfer_tickets: String,
    pub sendgrid_template_bn_cancel_transfer_tickets_receipt: String,
    pub sendgrid_template_bn_transfer_tickets: String,
    pub sendgrid_template_bn_transfer_tickets_receipt: String,
    pub sendgrid_template_bn_transfer_tickets_drip_source: String,
    pub sendgrid_template_bn_transfer_tickets_drip_destination: String,
    pub sendgrid_template_bn_user_invite: String,
    pub settlement_period_in_days: Option<u32>,
    pub spotify_auth_token: Option<String>,
    pub static_file_path: Option<String>,
    pub twilio_account_id: String,
    pub twilio_api_key: String,
    pub api_keys_encryption_key: String,
    pub jwt_expiry_time: Duration,
    pub branch_io_base_url: String,
    pub branch_io_branch_key: String,
    pub max_instances_per_ticket_type: i64,
    pub connection_pool: ConnectionPoolConfig,
    pub ssr_trigger_header: String,
    pub ssr_trigger_value: String,
    pub customer_io: CustomerIoSettings,
}

#[derive(Clone)]
pub struct Actix {
    pub workers: Option<usize>,
}

#[derive(Clone)]
pub struct ConnectionPoolConfig {
    pub min: u32,
    pub max: u32,
}

#[derive(Clone)]
pub struct CubeJs {
    pub secret: String,
}

#[derive(Clone)]
pub struct EmailTemplates {
    pub custom_broadcast: EmailTemplate,
    pub org_invite: EmailTemplate,
    pub password_reset: EmailTemplate,
    pub ticket_count_report: EmailTemplate,
    pub resend_download_link: EmailTemplate,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct EmailTemplate {
    pub provider: EmailProvider,
    pub template_id: String,
}

impl FromStr for EmailTemplate {
    type Err = BigNeonError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = val.split(':').collect_vec();
        if split.len() < 2 {
            return Err(ApplicationError::new(
                "Email template value was not in the correct format: '<provider_name>:<template_id>'".to_string(),
            )
            .into());
        }

        Ok(EmailTemplate {
            provider: split[0].parse()?,
            template_id: split[1].to_string(),
        })
    }
}

impl fmt::Display for EmailTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.provider, self.template_id)
    }
}

#[derive(Clone)]
pub struct CustomerIoSettings {
    pub base_url: String,
    pub api_key: String,
    pub site_id: String,
}

const CUSTOMER_IO_API_KEY: &str = "CUSTOMER_IO_API_KEY";
const CUSTOMER_IO_SITE_ID: &str = "CUSTOMER_IO_SITE_ID";
const CUSTOMER_IO_BASE_URL: &str = "CUSTOMER_IO_BASE_URL";
const CUBE_JS_SECRET: &str = "CUBE_JS_SECRET";
const ACTIX_WORKERS: &str = "ACTIX_WORKERS";
const ALLOWED_ORIGINS: &str = "ALLOWED_ORIGINS";
const APP_NAME: &str = "APP_NAME";
const API_HOST: &str = "API_HOST";
const API_PORT: &str = "API_PORT";
const DATABASE_URL: &str = "DATABASE_URL";
const REDIS_CONNECTION_STRING: &str = "REDIS_CONNECTION_STRING";
const REDIS_CONNECTION_TIMEOUT_MILLI: &str = "REDIS_CONNECTION_TIMEOUT_MILLI";
const REDIS_READ_TIMEOUT_MILLI: &str = "REDIS_READ_TIMEOUT_MILLI";
const REDIS_WRITE_TIMEOUT_MILLI: &str = "REDIS_WRITE_TIMEOUT_MILLI";
const REDIS_CACHE_PERIOD_MILLI: &str = "REDIS_CACHE_PERIOD_MILLI";
const READONLY_DATABASE_URL: &str = "READONLY_DATABASE_URL";
const DOMAIN: &str = "DOMAIN";
const EMAIL_TEMPLATES_CUSTOM_BROADCAST: &str = "EMAIL_TEMPLATES_CUSTOM_BROADCAST";
const EMAIL_TEMPLATES_ORG_INVITE: &str = "EMAIL_TEMPLATES_ORG_INVITE";
const EMAIL_TEMPLATES_PASSWORD_RESET: &str = "EMAIL_TEMPLATES_PASSWORD_RESET";
const EMAIL_TEMPLATES_TICKET_COUNT_REPORT: &str = "EMAIL_TEMPLATES_TICKET_COUNT_REPORT";
const EMAIL_TEMPLATES_RESEND_DOWNLOAD_LINK: &str = "EMAIL_TEMPLATES_RESEND_DOWNLOAD_LINK";
const ENVIRONMENT: &str = "ENVIRONMENT";
const FACEBOOK_APP_ID: &str = "FACEBOOK_APP_ID";
const FACEBOOK_APP_SECRET: &str = "FACEBOOK_APP_SECRET";
const GLOBEE_API_KEY: &str = "GLOBEE_API_KEY";
const GLOBEE_BASE_URL: &str = "GLOBEE_BASE_URL";
const VALIDATE_IPNS: &str = "VALIDATE_IPNS";
const API_BASE_URL: &str = "API_BASE_URL";
const GOOGLE_RECAPTCHA_SECRET_KEY: &str = "GOOGLE_RECAPTCHA_SECRET_KEY";
const PRIMARY_CURRENCY: &str = "PRIMARY_CURRENCY";
const STRIPE_SECRET_KEY: &str = "STRIPE_SECRET_KEY";
const STATIC_FILE_PATH: &str = "STATIC_FILE_PATH";
const TARI_URL: &str = "TARI_URL";
const TEST_DATABASE_URL: &str = "TEST_DATABASE_URL";
const TEST_READONLY_DATABASE_URL: &str = "TEST_READONLY_DATABASE_URL";
const TOKEN_SECRET: &str = "TOKEN_SECRET";
const TOKEN_ISSUER: &str = "TOKEN_ISSUER";
const HTTP_KEEP_ALIVE: &str = "HTTP_KEEP_ALIVE";
// Blocks all external communications from occurring
const BLOCK_EXTERNAL_COMMS: &str = "BLOCK_EXTERNAL_COMMS";
const FRONT_END_URL: &str = "FRONT_END_URL";

//Communication settings
const COMMUNICATION_DEFAULT_SOURCE_EMAIL: &str = "COMMUNICATION_DEFAULT_SOURCE_EMAIL";
const COMMUNICATION_DEFAULT_SOURCE_PHONE: &str = "COMMUNICATION_DEFAULT_SOURCE_PHONE";

//SendGrid settings
const SENDGRID_API_KEY: &str = "SENDGRID_API_KEY";
const SENDGRID_TEMPLATE_BN_REFUND: &str = "SENDGRID_TEMPLATE_BN_REFUND";
const SENDGRID_TEMPLATE_BN_USER_REGISTERED: &str = "SENDGRID_TEMPLATE_BN_USER_REGISTERED";
const SENDGRID_TEMPLATE_BN_PURCHASE_COMPLETED: &str = "SENDGRID_TEMPLATE_BN_PURCHASE_COMPLETED";
const SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_SOURCE: &str = "SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_SOURCE";
const SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_DESTINATION: &str =
    "SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_DESTINATION";
const SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS_RECEIPT: &str =
    "SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS_RECEIPT";
const SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS: &str = "SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS";
const SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_RECEIPT: &str = "SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_RECEIPT";
const SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS: &str = "SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS";
const SENDGRID_TEMPLATE_BN_USER_INVITE: &str = "SENDGRID_TEMPLATE_BN_USER_INVITE";

// Settlement period settings
const SETTLEMENT_PERIOD_IN_DAYS: &str = "SETTLEMENT_PERIOD_IN_DAYS";

//Spotify settings
const SPOTIFY_AUTH_TOKEN: &str = "SPOTIFY_AUTH_TOKEN";

const TWILIO_API_KEY: &str = "TWILIO_API_KEY";
const TWILIO_ACCOUNT_ID: &str = "TWILIO_ACCOUNT_ID";

const API_KEYS_ENCRYPTION_KEY: &str = "API_KEYS_ENCRYPTION_KEY";

const JWT_EXPIRY_TIME: &str = "JWT_EXPIRY_TIME";
const BRANCH_IO_BASE_URL: &str = "BRANCH_IO_BASE_URL";
const BRANCH_IO_BRANCH_KEY: &str = "BRANCH_IO_BRANCH_KEY";

const MAX_INSTANCES_PER_TICKET_TYPE: &str = "MAX_INSTANCES_PER_TICKET_TYPE";
const CONNECTION_POOL_MIN: &str = "CONNECTION_POOL_MIN";
const CONNECTION_POOL_MAX: &str = "CONNECTION_POOL_MAX";

const SSR_TRIGGER_HEADER: &str = "SSR_TRIGGER_HEADER";
const SSR_TRIGGER_VALUE: &str = "SSR_TRIGGER_VALUE";

fn get_env_var(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| panic!("{} must be defined", var))
}

impl Config {
    pub fn parse_environment() -> Result<Environment, EnumParseError> {
        if let Ok(environment_value) = env::var(&ENVIRONMENT) {
            return environment_value.parse();
        }
        // Default to development if not provided
        Ok(Environment::Development)
    }

    pub fn new(environment: Environment) -> Self {
        dotenv().ok();

        let app_name = env::var(&APP_NAME).unwrap_or_else(|_| "Big Neon".to_string());

        let redis_connection_string = match environment {
            Environment::Test => None,
            _ => env::var(&REDIS_CONNECTION_STRING).ok(),
        };
        let redis_connection_timeout = env::var(&REDIS_CONNECTION_TIMEOUT_MILLI)
            .ok()
            .map(|s| {
                s.parse()
                    .expect("Not a valid value for redis connection timeout in milliseconds")
            })
            .unwrap_or(50);
        let redis_read_timeout = env::var(&REDIS_READ_TIMEOUT_MILLI)
            .ok()
            .map(|s| {
                s.parse()
                    .expect("Not a valid value for redis read timeout in milliseconds")
            })
            .unwrap_or(50);
        let redis_write_timeout = env::var(&REDIS_WRITE_TIMEOUT_MILLI)
            .ok()
            .map(|s| {
                s.parse()
                    .expect("Not a valid value for redis write timeout in milliseconds")
            })
            .unwrap_or(50);
        let redis_cache_period: Option<usize> = env::var(&REDIS_CACHE_PERIOD_MILLI)
            .map(|p| p.parse().expect(&format!("{} is not a valid usize", ACTIX_WORKERS)))
            .ok();

        let database_url = match environment {
            Environment::Test => get_env_var(TEST_DATABASE_URL),
            _ => get_env_var(DATABASE_URL),
        };

        let readonly_database_url = match environment {
            Environment::Test => get_env_var(TEST_READONLY_DATABASE_URL),
            _ => env::var(&READONLY_DATABASE_URL).unwrap_or_else(|_| database_url.clone()),
        };

        let actix_workers: Option<usize> = env::var(&ACTIX_WORKERS)
            .map(|r| r.parse().expect(&format!("{} is not a valid usize", ACTIX_WORKERS)))
            .ok();
        let domain = env::var(&DOMAIN).unwrap_or_else(|_| "api.bigneon.com".to_string());

        let allowed_origins = env::var(&ALLOWED_ORIGINS).unwrap_or_else(|_| "*".to_string());
        let api_host = env::var(&API_HOST).unwrap_or_else(|_| "127.0.0.1".to_string());
        let api_port = env::var(&API_PORT).unwrap_or_else(|_| "8088".to_string());

        let secret = get_env_var(CUBE_JS_SECRET);
        let cube_js = CubeJs { secret };

        let primary_currency = env::var(&PRIMARY_CURRENCY).unwrap_or_else(|_| "usd".to_string());
        let stripe_secret_key = env::var(&STRIPE_SECRET_KEY).unwrap_or_else(|_| "<stripe not enabled>".to_string());

        let token_issuer = Box::new(DefaultTokenIssuer::new(
            get_env_var(TOKEN_SECRET),
            get_env_var(TOKEN_ISSUER),
        ));

        let facebook_app_id = env::var(&FACEBOOK_APP_ID).ok();

        let facebook_app_secret = env::var(&FACEBOOK_APP_SECRET).ok();

        let front_end_url = get_env_var(FRONT_END_URL);

        let tari_uri = get_env_var(TARI_URL);

        let tari_client = match environment {
            Environment::Test => Box::new(TariTestClient::new(tari_uri)) as Box<dyn TariClient + Send + Sync>,
            _ => {
                if tari_uri == "TEST" {
                    Box::new(TariTestClient::new(tari_uri)) as Box<dyn TariClient + Send + Sync>
                } else {
                    Box::new(HttpTariClient::new(tari_uri)) as Box<dyn TariClient + Send + Sync>
                }
            }
        };

        let globee_api_key = get_env_var(GLOBEE_API_KEY);
        let globee_base_url = env::var(&GLOBEE_BASE_URL).unwrap_or_else(|_| match environment {
            Environment::Production => "https://globee.com/payment-api/v1/".to_string(),
            _ => "https://test.globee.com/payment-api/v1/".to_string(),
        });

        let branch_io_base_url = env::var(&BRANCH_IO_BASE_URL).unwrap_or("https://api2.branch.io/v1".to_string());
        let branch_io_branch_key = get_env_var(BRANCH_IO_BRANCH_KEY);

        let api_base_url = get_env_var(API_BASE_URL);

        let validate_ipns = env::var(&VALIDATE_IPNS)
            .unwrap_or("true".to_string())
            .parse()
            .expect(&format!("{} is not a valid boolean value", VALIDATE_IPNS));
        let google_recaptcha_secret_key = env::var(&GOOGLE_RECAPTCHA_SECRET_KEY).ok();

        let communication_default_source_email = get_env_var(COMMUNICATION_DEFAULT_SOURCE_EMAIL);
        let communication_default_source_phone = get_env_var(COMMUNICATION_DEFAULT_SOURCE_PHONE);

        let email_templates = EmailTemplates {
            custom_broadcast: get_env_var(EMAIL_TEMPLATES_CUSTOM_BROADCAST).parse().unwrap(),
            org_invite: get_env_var(EMAIL_TEMPLATES_ORG_INVITE).parse().unwrap(),
            password_reset: get_env_var(EMAIL_TEMPLATES_PASSWORD_RESET).parse().unwrap(),
            ticket_count_report: get_env_var(EMAIL_TEMPLATES_TICKET_COUNT_REPORT).parse().unwrap(),
            resend_download_link: get_env_var(EMAIL_TEMPLATES_RESEND_DOWNLOAD_LINK).parse().unwrap(),
        };

        let customer_io_base_url = get_env_var(CUSTOMER_IO_BASE_URL);

        let customer_io_api_key = get_env_var(CUSTOMER_IO_API_KEY);

        let customer_io_site_id = get_env_var(CUSTOMER_IO_SITE_ID);

        let customer_io = CustomerIoSettings {
            base_url: customer_io_base_url,
            api_key: customer_io_api_key,
            site_id: customer_io_site_id,
        };

        let sendgrid_api_key = get_env_var(SENDGRID_API_KEY);
        let sendgrid_template_bn_refund = get_env_var(SENDGRID_TEMPLATE_BN_REFUND);

        let sendgrid_template_bn_user_registered = get_env_var(SENDGRID_TEMPLATE_BN_USER_REGISTERED);

        let sendgrid_template_bn_purchase_completed = get_env_var(SENDGRID_TEMPLATE_BN_PURCHASE_COMPLETED);
        let sendgrid_template_bn_transfer_tickets = get_env_var(SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS);
        let sendgrid_template_bn_transfer_tickets_receipt = get_env_var(SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_RECEIPT);
        let sendgrid_template_bn_transfer_tickets_drip_destination =
            get_env_var(SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_DESTINATION);
        let sendgrid_template_bn_transfer_tickets_drip_source =
            get_env_var(SENDGRID_TEMPLATE_BN_TRANSFER_TICKETS_DRIP_SOURCE);
        let sendgrid_template_bn_cancel_transfer_tickets = get_env_var(SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS);
        let sendgrid_template_bn_cancel_transfer_tickets_receipt =
            get_env_var(SENDGRID_TEMPLATE_BN_CANCEL_TRANSFER_TICKETS_RECEIPT);
        let sendgrid_template_bn_user_invite = get_env_var(SENDGRID_TEMPLATE_BN_USER_INVITE);

        // Force settlement period in days to 1 for testing
        let settlement_period_in_days = if environment == Environment::Test {
            Some(1)
        } else {
            env::var(&SETTLEMENT_PERIOD_IN_DAYS)
                .ok()
                .map(|s| s.parse().expect("Not a valid integer for settlement period in days"))
        };

        let spotify_auth_token = env::var(&SPOTIFY_AUTH_TOKEN).ok();

        let twilio_api_key = get_env_var(TWILIO_API_KEY);

        let twilio_account_id = get_env_var(TWILIO_ACCOUNT_ID);

        let api_keys_encryption_key = get_env_var(API_KEYS_ENCRYPTION_KEY);

        let block_external_comms = match env::var(&BLOCK_EXTERNAL_COMMS)
            .unwrap_or_else(|_| "0".to_string())
            .as_str()
        {
            "0" => false,
            _ => true,
        };

        let http_keep_alive = env::var(&HTTP_KEEP_ALIVE).unwrap_or("75".to_string()).parse().unwrap();

        let jwt_expiry_time =
            Duration::minutes(env::var(&JWT_EXPIRY_TIME).unwrap_or("15".to_string()).parse().unwrap());

        let max_instances_per_ticket_type = env::var(&MAX_INSTANCES_PER_TICKET_TYPE)
            .map(|s| {
                s.parse()
                    .expect("Not a valid integer for max instances per ticket type")
            })
            .unwrap_or(10000);
        let connection_pool = ConnectionPoolConfig {
            min: env::var(CONNECTION_POOL_MIN)
                .map(|s| s.parse().expect("Not a valid integer for CONNECTION_POOL_MIN"))
                .unwrap_or(1),
            max: env::var(CONNECTION_POOL_MAX)
                .map(|s| s.parse().expect("Not a valid integer for CONNECTION_POOL_MAX"))
                .unwrap_or(20),
        };

        let ssr_trigger_header = env::var(&SSR_TRIGGER_HEADER).unwrap_or("x-ssr".to_string());
        let ssr_trigger_value = env::var(&SSR_TRIGGER_VALUE).unwrap_or("facebook".to_string());

        let static_file_path = env::var(&STATIC_FILE_PATH).map(|s| Some(s)).unwrap_or(None);

        Config {
            actix: Actix { workers: actix_workers },
            customer_io,
            allowed_origins,
            app_name,
            api_host,
            api_port,
            cube_js,
            database_url,
            redis_connection_string,
            redis_connection_timeout,
            redis_read_timeout,
            redis_write_timeout,
            redis_cache_period,
            readonly_database_url,
            domain,
            email_templates,
            environment,
            facebook_app_id,
            facebook_app_secret,
            globee_api_key,
            globee_base_url,
            branch_io_base_url,
            validate_ipns,
            api_base_url,
            google_recaptcha_secret_key,
            http_keep_alive,
            block_external_comms,
            primary_currency,
            stripe_secret_key,
            token_issuer,
            front_end_url,
            tari_client,
            communication_default_source_email,
            communication_default_source_phone,
            sendgrid_api_key,
            sendgrid_template_bn_refund,
            sendgrid_template_bn_user_registered,
            sendgrid_template_bn_purchase_completed,
            sendgrid_template_bn_cancel_transfer_tickets,
            sendgrid_template_bn_cancel_transfer_tickets_receipt,
            sendgrid_template_bn_transfer_tickets,
            sendgrid_template_bn_transfer_tickets_receipt,
            sendgrid_template_bn_transfer_tickets_drip_destination,
            sendgrid_template_bn_transfer_tickets_drip_source,
            sendgrid_template_bn_user_invite,
            settlement_period_in_days,
            spotify_auth_token,
            static_file_path,
            twilio_api_key,
            twilio_account_id,
            api_keys_encryption_key,
            jwt_expiry_time,
            branch_io_branch_key,
            max_instances_per_ticket_type,
            connection_pool,
            ssr_trigger_header,
            ssr_trigger_value,
        }
    }
}
