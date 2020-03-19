use crate::payments::*;
use chrono::NaiveDateTime;
use db::models::PaymentProviders;
use uuid::Uuid;

pub enum PaymentProcessorBehavior {
    AuthThenComplete(Box<dyn AuthThenCompletePaymentBehavior>),
    RedirectToPaymentPage(Box<dyn RedirectToPaymentPageBehavior>),
}

#[async_trait::async_trait]
pub trait AuthThenCompletePaymentBehavior {
    fn payment_provider(&self) -> PaymentProviders;

    async fn create_token_for_repeat_charges(
        &self,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError>;

    async fn update_repeat_token(
        &self,
        repeat_token: &str,
        token: &str,
        description: &str,
    ) -> Result<RepeatChargeToken, PaymentProcessorError>;

    async fn auth(
        &self,
        token: &str,
        amount: i64,
        currency: &str,
        description: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<ChargeAuthResult, PaymentProcessorError>;

    async fn complete_authed_charge(&self, auth_token: &str) -> Result<ChargeResult, PaymentProcessorError>;
}

#[async_trait::async_trait]
pub trait RedirectToPaymentPageBehavior {
    fn payment_provider(&self) -> PaymentProviders;
    async fn create_payment_request(
        &self,
        total: f64,
        email: String,
        order_id: Uuid,
        ipn_url: Option<String>,

        success_url: Option<String>,
        cancel_url: Option<String>,
    ) -> Result<RedirectInfo, PaymentProcessorError>;
}

#[async_trait::async_trait]
pub trait PaymentProcessor {
    fn behavior(&self) -> PaymentProcessorBehavior;
    async fn refund(&self, auth_token: &str) -> Result<ChargeAuthResult, PaymentProcessorError>;

    async fn partial_refund(&self, auth_token: &str, amount: i64) -> Result<ChargeAuthResult, PaymentProcessorError>;

    fn partial_refund_blocking(&self, auth_token: &str, amount: i64)
        -> Result<ChargeAuthResult, PaymentProcessorError>;

    async fn update_metadata(
        &self,
        charge_id: &str,
        metadata: Vec<(String, String)>,
    ) -> Result<UpdateMetadataResult, PaymentProcessorError>;
}

#[derive(Serialize, Clone)]
pub struct RedirectInfo {
    pub redirect_url: String,
    pub id: String,
    pub expires_at: NaiveDateTime,
}
